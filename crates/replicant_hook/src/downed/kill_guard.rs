use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

use replicant_game::Replicant;
use replicant_game::offsets::PLAYER_FORCE_KILL_FUNCTION_RVA;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::downed::tick::kill_is_blocked_now;
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;

static ORIGINAL_FORCE_KILL: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
static OWN_KILL_RUNNING: AtomicBool = AtomicBool::new(false);
static BLOCKED_KILL_SEEN: AtomicBool = AtomicBool::new(false);

type ForceKillFn = unsafe extern "system" fn(*mut c_void);

pub(super) struct OwnKill;

impl OwnKill {
    pub(super) fn begin() -> Self {
        OWN_KILL_RUNNING.store(true, Ordering::Relaxed);
        Self
    }
}

impl Drop for OwnKill {
    fn drop(&mut self) {
        OWN_KILL_RUNNING.store(false, Ordering::Relaxed);
    }
}

pub fn install(build: VerifiedBuild) {
    if BUILD.set(build).is_err() {
        return;
    }
    #[expect(
        unsafe_code,
        reason = "hooks ActorPlayable::ForceKill by its address to veto the local player's instant kill"
    )]
    // SAFETY: the RVA starts ActorPlayable::ForceKill (this in rcx, void) in the verified build
    // (docs off-hand/player-combat.md, the player vtable slot 0x580) and the replacement matches.
    unsafe {
        hook_address(
            "ActorPlayable::ForceKill",
            (build.module_base() + u64::from(PLAYER_FORCE_KILL_FUNCTION_RVA)) as usize,
            force_kill_detour as *const () as *mut c_void,
            &ORIGINAL_FORCE_KILL,
        );
    }
}

pub fn take_blocked_kill() -> bool {
    BLOCKED_KILL_SEEN.swap(false, Ordering::Relaxed)
}

#[expect(
    unsafe_code,
    reason = "calls the original ForceKill through the trampoline MinHook built"
)]
extern "system" fn force_kill_detour(actor: *mut c_void) {
    let original = ORIGINAL_FORCE_KILL.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    if panic::catch_unwind(|| should_block(actor as usize as u64)).unwrap_or(false) {
        BLOCKED_KILL_SEEN.store(true, Ordering::Relaxed);
        log_line("kill: blocked an instant kill of your player, the partner is still up");
        return;
    }
    // SAFETY: the trampoline continues into the original ForceKill, which has this signature.
    unsafe { core::mem::transmute::<*mut c_void, ForceKillFn>(original)(actor) };
}

fn should_block(actor: u64) -> bool {
    if OWN_KILL_RUNNING.load(Ordering::Relaxed) || !kill_is_blocked_now() {
        return false;
    }
    let Some(build) = BUILD.get() else {
        return false;
    };
    panic::catch_unwind(|| {
        Replicant::new(InProcessMemory::new(build))
            .playable_actor()
            .is_ok_and(|player| player == actor)
    })
    .unwrap_or(false)
}
