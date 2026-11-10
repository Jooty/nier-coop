use core::ffi::c_void;
use core::ptr;
use std::collections::HashSet;
use std::panic;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{LazyLock, Mutex, OnceLock, PoisonError};

use replicant_game::npcs::read_npc_event;
use replicant_game::offsets::HUMAN_NPC_STATE_STEP_FUNCTION_RVA;

use crate::build_gate::VerifiedBuild;
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;

static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
static ORIGINAL_STATE_STEP: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static HELD_NPCS: LazyLock<Mutex<HashSet<u64>>> = LazyLock::new(Mutex::default);

type StateStep = unsafe extern "system" fn(*mut c_void, f32) -> u8;

pub fn install(build: VerifiedBuild) {
    super::talk_guard_hook::install(build);
    if HOOKED_BUILD.set(build).is_err() {
        return;
    }
    #[expect(
        unsafe_code,
        reason = "hooks the villager state machine step by address"
    )]
    // SAFETY: the RVA starts CActorHumanNpc's state step (vtable slot 0x4e8: actor in rcx, delta
    // time in xmm1, bool in al) in the verified build, which the replacement mirrors.
    unsafe {
        hook_address(
            "CActorHumanNpc_StateStep",
            (build.module_base() + u64::from(HUMAN_NPC_STATE_STEP_FUNCTION_RVA)) as usize,
            npc_state_step_detour as *const () as *mut c_void,
            &ORIGINAL_STATE_STEP,
        );
    }
}

pub(super) fn is_installed() -> bool {
    !ORIGINAL_STATE_STEP.load(Ordering::Acquire).is_null()
}

pub(super) fn set_held(actors: HashSet<u64>) {
    *HELD_NPCS.lock().unwrap_or_else(PoisonError::into_inner) = actors;
}

fn skips_ai(actor: u64) -> bool {
    let held = HELD_NPCS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .contains(&actor);
    // A talk or script this game started on the NPC always runs, even within the tick it began.
    held && HOOKED_BUILD.get().is_some_and(|build| {
        read_npc_event(&InProcessMemory::new(build), actor).is_ok_and(|event| event.is_none())
    })
}

#[expect(
    unsafe_code,
    reason = "calls the original state step through the trampoline MinHook built"
)]
extern "system" fn npc_state_step_detour(actor: *mut c_void, delta_seconds: f32) -> u8 {
    // A false return makes the actor manager destroy the actor, so a held NPC answers true.
    if panic::catch_unwind(|| skips_ai(actor as u64)).unwrap_or(false) {
        return 1;
    }
    let original = ORIGINAL_STATE_STEP.load(Ordering::Acquire);
    if original.is_null() {
        return 1;
    }
    // SAFETY: the trampoline continues into the original step, which has this exact signature.
    unsafe { core::mem::transmute::<*mut c_void, StateStep>(original)(actor, delta_seconds) }
}
