use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{Mutex, PoisonError};

use rc_game::dummy_hits::{
    DummyHitCallbacks, PUT_ACTOR_DAMAGE_DUMMY_NATIVE_RVA, dummy_callback_name, dummy_hit_log_line,
    dummy_native_args_len,
};
use rc_game::{PhaseEntry, Replicant};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;
use crate::story_relay::state::HOOKED_BUILD;

static CALLBACKS: Mutex<DummyHitCallbacks> = Mutex::new(DummyHitCallbacks::new());
static ORIGINAL_PUT_ACTOR_DAMAGE_DUMMY: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

type LuaNative = unsafe extern "system" fn(*mut c_void);

pub(super) fn install(build: VerifiedBuild) {
    #[expect(
        unsafe_code,
        reason = "hooks the _PutActorDamageDummy script native by address to learn its callback names"
    )]
    // SAFETY: the RVA starts _PutActorDamageDummy in the verified build (binding table 0x1306710,
    // thunk 0x6E10C0), which takes the Lua argument block as its only argument.
    unsafe {
        hook_address(
            "LuaNative::PutActorDamageDummy",
            (build.module_base() + u64::from(PUT_ACTOR_DAMAGE_DUMMY_NATIVE_RVA)) as usize,
            put_actor_damage_dummy_detour as *const () as *mut c_void,
            &ORIGINAL_PUT_ACTOR_DAMAGE_DUMMY,
        );
    }
}

pub(super) fn is_registered(phase: u16, callback: &str) -> bool {
    CALLBACKS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .is_registered(phase, callback)
}

pub(super) fn on_fire(phase: u16, function_name: &str) {
    let runs_here = CALLBACKS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .count_run(phase, function_name);
    log_line(&dummy_hit_log_line(function_name, runs_here));
}

#[expect(
    unsafe_code,
    reason = "copies the native's argument block and calls the original through the trampoline"
)]
extern "system" fn put_actor_damage_dummy_detour(args: *mut c_void) {
    let args_address = args as u64;
    let name = panic::catch_unwind(|| read_callback_name(args_address)).ok();
    let trampoline = ORIGINAL_PUT_ACTOR_DAMAGE_DUMMY.load(Ordering::Acquire);
    if !trampoline.is_null() {
        // SAFETY: the trampoline continues into the original native, which has this signature.
        unsafe { core::mem::transmute::<*mut c_void, LuaNative>(trampoline)(args) };
    }
    if let Some(name) = name {
        let _ = panic::catch_unwind(|| note_registered(name));
    }
}

#[expect(
    unsafe_code,
    reason = "reads the native's argument block, which the native reads itself"
)]
fn read_callback_name(args_address: u64) -> Result<String, String> {
    let mut block = vec![0_u8; dummy_native_args_len()];
    // SAFETY: the native reads the same bytes (its first three arguments) from this block.
    unsafe {
        ptr::copy_nonoverlapping(args_address as *const u8, block.as_mut_ptr(), block.len());
    }
    dummy_callback_name(&block).map_err(|refusal| format!("{refusal:?}"))
}

fn note_registered(name: Result<String, String>) {
    let name = match name {
        Ok(name) => name,
        Err(refusal) => {
            log_line(&format!(
                "dummy hit: _PutActorDamageDummy callback name unreadable ({refusal})"
            ));
            return;
        }
    };
    let Some(build) = HOOKED_BUILD.get() else {
        return;
    };
    let game = Replicant::new(InProcessMemory::new(build));
    let phase = game
        .current_phase()
        .ok()
        .as_deref()
        .and_then(PhaseEntry::index_of)
        .and_then(|index| u16::try_from(index).ok());
    let Some(phase) = phase else {
        log_line(&format!(
            "dummy hit: {name} not recorded, the phase is unreadable"
        ));
        return;
    };
    let recorded = CALLBACKS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .register(phase, &name);
    log_line(&format!(
        "dummy hit: _PutActorDamageDummy callback {name} recorded for phase {phase} ({})",
        if recorded { "ok" } else { "table full" }
    ));
}
