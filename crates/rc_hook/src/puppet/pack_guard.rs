use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicPtr, AtomicU64, Ordering};

use rc_game::Replicant;
use rc_game::offsets::MODEL_PACK_RELEASE_FUNCTION_RVA;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;
use crate::puppet::game_calls::GameCalls;

static ORIGINAL_RELEASE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static INSTALLED: OnceLock<Option<VerifiedBuild>> = OnceLock::new();
static GUARDED_PACK: AtomicU64 = AtomicU64::new(0);
static GUARDED_ACTOR: AtomicU64 = AtomicU64::new(0);

type ReleaseModelPack = unsafe extern "system" fn(*mut c_void, *mut c_void, u8);

pub(super) fn ensure_pack_guard(build: &VerifiedBuild) -> bool {
    // Without the hook a pack could be freed under the weapon actor: then no weapon is shown.
    INSTALLED
        .get_or_init(|| {
            let replacement = release_detour as *const () as *mut c_void;
            let address =
                (build.module_base() + u64::from(MODEL_PACK_RELEASE_FUNCTION_RVA)) as usize;
            #[expect(unsafe_code, reason = "hooks ModelPackManager::Release by its address")]
            // SAFETY: the RVA starts the release (manager, pack, force) in the verified build, and
            // the replacement has the same signature.
            let hooked = unsafe {
                hook_address(
                    "ModelPackManager::Release",
                    address,
                    replacement,
                    &ORIGINAL_RELEASE,
                )
            };
            hooked.then_some(*build)
        })
        .is_some()
}

pub(super) fn guard(actor: u64, pack: u64) {
    GUARDED_ACTOR.store(actor, Ordering::Release);
    GUARDED_PACK.store(pack, Ordering::Release);
}

pub(super) fn still_guarded(actor: u64) -> bool {
    GUARDED_ACTOR.load(Ordering::Acquire) == actor
}

pub(super) fn unguard(actor: u64) {
    if GUARDED_ACTOR
        .compare_exchange(actor, 0, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
    {
        GUARDED_PACK.store(0, Ordering::Release);
    }
}

fn drop_guarded_actor(pack: u64) {
    if pack == 0 || GUARDED_PACK.load(Ordering::Acquire) != pack {
        return;
    }
    let actor = GUARDED_ACTOR.swap(0, Ordering::AcqRel);
    GUARDED_PACK.store(0, Ordering::Release);
    let Some(Some(build)) = INSTALLED.get() else {
        return;
    };
    if actor == 0 {
        return;
    }
    let game = Replicant::new(InProcessMemory::new(build));
    let calls = GameCalls::new(build);
    if let Err(error) = calls.detach_model(game.memory(), actor) {
        log_line(&format!(
            "puppet weapon: cannot detach before release ({error})"
        ));
    }
    calls.destroy(actor);
    log_line("puppet weapon: the game released its pack, weapon actor removed first");
}

#[expect(
    unsafe_code,
    reason = "calls the original release through the trampoline MinHook built"
)]
extern "system" fn release_detour(manager: *mut c_void, pack: *mut c_void, force: u8) {
    let _ = panic::catch_unwind(|| drop_guarded_actor(pack as usize as u64));
    let original = ORIGINAL_RELEASE.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original release, which has this signature.
    unsafe {
        core::mem::transmute::<*mut c_void, ReleaseModelPack>(original)(manager, pack, force);
    }
}
