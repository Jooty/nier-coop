use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

use rc_game::Replicant;
use rc_game::offsets::{PLAYER_PARAM_ARRAY_RVA, PLAYER_PARAM_SET_HEALTH_FUNCTION_RVA};

use crate::build_gate::VerifiedBuild;
use crate::game_hooks::hook_slot::hook_address;
use crate::memory::InProcessMemory;

static ENABLED: AtomicBool = AtomicBool::new(false);
static LETHAL_HIT_SEEN: AtomicBool = AtomicBool::new(false);
static ORIGINAL_SET_HEALTH: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();

type SetHealth = unsafe extern "system" fn(*mut c_void, i32);

pub(super) fn install(build: VerifiedBuild) {
    if HOOKED_BUILD.set(build).is_err() {
        return;
    }
    let replacement = set_health_detour as *const () as *mut c_void;
    let address = (build.module_base() + u64::from(PLAYER_PARAM_SET_HEALTH_FUNCTION_RVA)) as usize;
    #[expect(unsafe_code, reason = "hooks CPlayerParam::SetHp by its address")]
    // SAFETY: the RVA starts CPlayerParam::SetHp (this in rcx, new health in edx, void) in the
    // verified build, and the replacement has the same signature.
    unsafe {
        hook_address(
            "CPlayerParam::SetHp",
            address,
            replacement,
            &ORIGINAL_SET_HEALTH,
        )
    };
}

pub fn set_enabled(enabled: bool) {
    ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn take_lethal_hit_seen() -> bool {
    LETHAL_HIT_SEEN.swap(false, Ordering::Relaxed)
}

#[expect(
    unsafe_code,
    reason = "calls the original SetHp through the trampoline MinHook built"
)]
pub fn set_local_health_through_game(value: i32) -> bool {
    let original = ORIGINAL_SET_HEALTH.load(Ordering::Acquire);
    let Some(build) = HOOKED_BUILD.get() else {
        return false;
    };
    if original.is_null() {
        return false;
    }
    let player_param = (build.module_base() + u64::from(PLAYER_PARAM_ARRAY_RVA)) as usize;
    // SAFETY: the trampoline continues into the original SetHp (this, new health, void); the
    // array start is slot 0, the local player's CPlayerParam; this runs on the game thread.
    unsafe {
        core::mem::transmute::<*mut c_void, SetHealth>(original)(
            player_param as *mut c_void,
            value,
        );
    }
    true
}

pub fn is_enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

pub fn is_installed() -> bool {
    !ORIGINAL_SET_HEALTH.load(Ordering::Acquire).is_null()
}

#[expect(
    unsafe_code,
    reason = "calls the original SetHp through the trampoline MinHook built"
)]
extern "system" fn set_health_detour(player_param: *mut c_void, requested_health: i32) {
    let original = ORIGINAL_SET_HEALTH.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    if is_lethal_to_local_player(player_param, requested_health) {
        LETHAL_HIT_SEEN.store(true, Ordering::Relaxed);
    }
    if ENABLED.load(Ordering::Relaxed) && would_hurt_local_player(player_param, requested_health) {
        return;
    }
    // SAFETY: the trampoline continues into the original SetHp, which has this exact signature.
    unsafe {
        core::mem::transmute::<*mut c_void, SetHealth>(original)(player_param, requested_health);
    }
}

fn would_hurt_local_player(player_param: *mut c_void, requested_health: i32) -> bool {
    let Some(build) = HOOKED_BUILD.get() else {
        return false;
    };
    let address = player_param as usize as u64;
    panic::catch_unwind(|| {
        Replicant::new(InProcessMemory::new(build))
            .health_write_hurts_local_player(address, requested_health)
            .unwrap_or(false)
    })
    .unwrap_or(false)
}

fn is_lethal_to_local_player(player_param: *mut c_void, requested_health: i32) -> bool {
    let Some(build) = HOOKED_BUILD.get() else {
        return false;
    };
    let address = player_param as usize as u64;
    panic::catch_unwind(|| {
        Replicant::new(InProcessMemory::new(build))
            .health_write_is_lethal_to_local_player(address, requested_health)
            .unwrap_or(false)
    })
    .unwrap_or(false)
}
