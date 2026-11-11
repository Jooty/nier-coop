use core::cell::Cell;
use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::PoisonError;
use std::sync::atomic::{AtomicPtr, AtomicU32, Ordering};

use replicant_game::GameMemory;
use replicant_game::fight_sync::{
    ArmedBy, LUA_STEP_RVA, PopSource, QUEUED_CALL_NAME_CAPACITY, QUEUED_CALL_NAME_OFFSET,
    pop_source, queued_call_name,
};
use replicant_game::story_relay::{MARK_QUEUED_CALL_READY_RVA, QUEUED_CALL_ARGUMENTS_OFFSET};

use crate::build_gate::VerifiedBuild;
use crate::fight_sync::state::{BUILD, OBJECT_ARMED};
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;

static ORIGINAL_LUA_STEP: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_MARK_QUEUED_CALL_READY: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static OWN_ARMS_RUNNING: AtomicU32 = AtomicU32::new(0);

thread_local! {
    static LUA_STEP_DEPTH_ON_THIS_THREAD: Cell<u32> = const { Cell::new(0) };
}

type LuaStep = unsafe extern "system" fn(u64, u64) -> u64;
type MarkQueuedCallReady = unsafe extern "system" fn(u64, u64);

/// Held while this hook's own relay paths mark a queued call ready, so the arm is not taken for
/// a game object's.
pub(crate) struct OwnArm;

impl OwnArm {
    pub(crate) fn begin() -> Self {
        OWN_ARMS_RUNNING.fetch_add(1, Ordering::AcqRel);
        Self
    }
}

impl Drop for OwnArm {
    fn drop(&mut self) {
        OWN_ARMS_RUNNING.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Held for one script step on this thread; the drop lowers the depth again even when the step
/// unwinds, so a ready mark later on is never taken for the script's.
struct LuaStepRunning;

impl LuaStepRunning {
    fn begin() -> Self {
        LUA_STEP_DEPTH_ON_THIS_THREAD.with(|depth| depth.set(depth.get().saturating_add(1)));
        Self
    }

    fn is_running_here() -> bool {
        LUA_STEP_DEPTH_ON_THIS_THREAD.with(|depth| depth.get() > 0)
    }
}

impl Drop for LuaStepRunning {
    fn drop(&mut self) {
        LUA_STEP_DEPTH_ON_THIS_THREAD.with(|depth| depth.set(depth.get().saturating_sub(1)));
    }
}

pub(super) fn install(build: VerifiedBuild) {
    let base = build.module_base();
    #[expect(
        unsafe_code,
        reason = "hooks the script step and the queued call ready marker by address"
    )]
    // SAFETY: the RVAs start ScriptManager::LuaStep (two pointer arguments, returns in rax) and
    // MarkQueuedReady (manager, slot) in the verified build (docs symbols.tsv); the replacements
    // pass every argument and the return value through unchanged.
    unsafe {
        hook_address(
            "ScriptManager::LuaStep",
            (base + u64::from(LUA_STEP_RVA)) as usize,
            lua_step_detour as *const () as *mut c_void,
            &ORIGINAL_LUA_STEP,
        );
        hook_address(
            "ScriptCallbacks::MarkQueuedReady",
            (base + u64::from(MARK_QUEUED_CALL_READY_RVA)) as usize,
            mark_queued_call_ready_detour as *const () as *mut c_void,
            &ORIGINAL_MARK_QUEUED_CALL_READY,
        );
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original script step through the trampoline"
)]
extern "system" fn lua_step_detour(context: u64, step: u64) -> u64 {
    let trampoline = ORIGINAL_LUA_STEP.load(Ordering::Acquire);
    if trampoline.is_null() {
        return 0;
    }
    let _running = LuaStepRunning::begin();
    // SAFETY: the trampoline continues into the original function with this signature.
    unsafe { core::mem::transmute::<*mut c_void, LuaStep>(trampoline)(context, step) }
}

#[expect(
    unsafe_code,
    reason = "calls the original ready marker through the trampoline"
)]
extern "system" fn mark_queued_call_ready_detour(manager: u64, slot: u64) {
    let trampoline = ORIGINAL_MARK_QUEUED_CALL_READY.load(Ordering::Acquire);
    if trampoline.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original function with this signature.
    unsafe { core::mem::transmute::<*mut c_void, MarkQueuedCallReady>(trampoline)(manager, slot) };
    let _ = panic::catch_unwind(|| note_arm(manager, slot));
}

fn note_arm(manager: u64, slot: u64) {
    let Some(build) = BUILD.get() else {
        return;
    };
    let arguments = slot.wrapping_add(QUEUED_CALL_ARGUMENTS_OFFSET);
    if slot == 0 || pop_source(manager, arguments) != Some(PopSource::Queued) {
        return;
    }
    let by = if LuaStepRunning::is_running_here() || OWN_ARMS_RUNNING.load(Ordering::Acquire) > 0 {
        ArmedBy::ScriptOrRelay
    } else {
        ArmedBy::GameObject
    };
    let name = InProcessMemory::new(build)
        .read_array::<QUEUED_CALL_NAME_CAPACITY>(slot + QUEUED_CALL_NAME_OFFSET)
        .ok()
        .and_then(|bytes| queued_call_name(&bytes))
        .unwrap_or_default();
    OBJECT_ARMED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .armed(arguments, &name, by);
}

pub(super) fn armed_by_object(arguments: u64, name: &str) -> bool {
    // True when a game object armed this queued call under the same name.
    OBJECT_ARMED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .popped(arguments, name)
}

pub(super) fn forget_object_arms() {
    OBJECT_ARMED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clear();
}
