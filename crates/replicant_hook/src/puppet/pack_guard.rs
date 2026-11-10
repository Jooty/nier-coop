use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU64, Ordering};

use replicant_game::Replicant;
use replicant_game::game_closing::actors_to_destroy;
use replicant_game::offsets::MODEL_PACK_RELEASE_FUNCTION_RVA;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;
use crate::puppet::game_calls::GameCalls;

static ORIGINAL_RELEASE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static GAME_CLOSING: AtomicBool = AtomicBool::new(false);
static INSTALLED: OnceLock<Option<VerifiedBuild>> = OnceLock::new();
const GUARD_SLOTS: usize = 8;
static GUARDED_PACKS: [AtomicU64; GUARD_SLOTS] = [const { AtomicU64::new(0) }; GUARD_SLOTS];
static GUARDED_ACTORS: [AtomicU64; GUARD_SLOTS] = [const { AtomicU64::new(0) }; GUARD_SLOTS];

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

pub(crate) fn mark_game_closing() {
    GAME_CLOSING.store(true, Ordering::Release);
}

pub(super) fn has_free_slot() -> bool {
    slot_of(0).is_some()
}

pub(super) fn guard(actor: u64, pack: u64) -> bool {
    let slot = slot_of(actor).or_else(|| slot_of(0));
    let Some(slot) = slot else {
        log_line("puppet pack guard: no free slot, the actor is not guarded");
        return false;
    };
    GUARDED_ACTORS[slot].store(actor, Ordering::Release);
    GUARDED_PACKS[slot].store(pack, Ordering::Release);
    true
}

pub(super) fn still_guarded(actor: u64) -> bool {
    actor != 0 && slot_of(actor).is_some()
}

pub(super) fn unguard(actor: u64) {
    if let Some(slot) = slot_of(actor) {
        GUARDED_ACTORS[slot].store(0, Ordering::Release);
        GUARDED_PACKS[slot].store(0, Ordering::Release);
    }
}

fn slot_of(actor: u64) -> Option<usize> {
    GUARDED_ACTORS
        .iter()
        .position(|guarded| guarded.load(Ordering::Acquire) == actor)
}

fn drop_guarded_actors(pack: u64) {
    if pack == 0 {
        return;
    }
    let mut guarded = Vec::new();
    for slot in 0..GUARD_SLOTS {
        if GUARDED_PACKS[slot].load(Ordering::Acquire) == pack {
            guarded.push(GUARDED_ACTORS[slot].swap(0, Ordering::AcqRel));
            GUARDED_PACKS[slot].store(0, Ordering::Release);
        }
    }
    for actor in actors_to_destroy(GAME_CLOSING.load(Ordering::Acquire), guarded) {
        drop_guarded_actor(actor);
    }
}

fn drop_guarded_actor(actor: u64) {
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
            "puppet pack guard: cannot detach before release ({error})"
        ));
    }
    calls.destroy(actor);
    log_line("puppet pack guard: the game released a pack, the actor wearing it was removed first");
}

#[expect(
    unsafe_code,
    reason = "calls the original release through the trampoline MinHook built"
)]
extern "system" fn release_detour(manager: *mut c_void, pack: *mut c_void, force: u8) {
    let _ = panic::catch_unwind(|| drop_guarded_actors(pack as usize as u64));
    let original = ORIGINAL_RELEASE.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original release, which has this signature.
    unsafe {
        core::mem::transmute::<*mut c_void, ReleaseModelPack>(original)(manager, pack, force);
    }
}
