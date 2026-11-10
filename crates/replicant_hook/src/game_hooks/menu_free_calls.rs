use core::ffi::c_void;
use core::ptr;
use std::sync::atomic::{AtomicPtr, AtomicU32, Ordering};

use replicant_game::offsets::{ACTOR_HIT_POINT_MESSAGE_FUNCTION_RVA, HUD_UPDATE_FUNCTION_RVA};

use crate::build_gate::VerifiedBuild;
use crate::paused_hits::paused_players_can_be_hit;
use nc_hook::detour::hook_address;

const HIT_POINT_DAMAGE_MESSAGE: i32 = 9;
const HIT_POINT_HEAL_MESSAGE: i32 = 0xB;

static ORIGINAL_HIT_POINT_MESSAGE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_HUD_UPDATE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static MENU_FREE_CALLS_ACTIVE: AtomicU32 = AtomicU32::new(0);

type HitPointMessage = unsafe extern "system" fn(*mut c_void, i32, *mut c_void, u64, i64) -> u64;
type HudUpdate = unsafe extern "system" fn(*mut c_void, f32);

struct MenuFreeCall;

impl MenuFreeCall {
    fn begin() -> Self {
        MENU_FREE_CALLS_ACTIVE.fetch_add(1, Ordering::Relaxed);
        Self
    }
}

impl Drop for MenuFreeCall {
    fn drop(&mut self) {
        MENU_FREE_CALLS_ACTIVE.fetch_sub(1, Ordering::Relaxed);
    }
}

pub(super) fn menu_free_call_active() -> bool {
    MENU_FREE_CALLS_ACTIVE.load(Ordering::Relaxed) != 0
}

pub(super) fn install(build: VerifiedBuild) {
    let base = build.module_base();
    #[expect(unsafe_code, reason = "hooks two game functions by address")]
    // SAFETY: both RVAs start the named functions in the verified build, and each replacement
    // has the signature of the function it replaces.
    unsafe {
        hook_address(
            "Actor_HitPointMessage",
            (base + u64::from(ACTOR_HIT_POINT_MESSAGE_FUNCTION_RVA)) as usize,
            hit_point_message_detour as *const () as *mut c_void,
            &ORIGINAL_HIT_POINT_MESSAGE,
        );
        hook_address(
            "Hud_Update",
            (base + u64::from(HUD_UPDATE_FUNCTION_RVA)) as usize,
            hud_update_detour as *const () as *mut c_void,
            &ORIGINAL_HUD_UPDATE,
        );
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original message handler through the trampoline MinHook built"
)]
extern "system" fn hit_point_message_detour(
    actor: *mut c_void,
    message: i32,
    source: *mut c_void,
    amount: u64,
    extra: i64,
) -> u64 {
    let original = ORIGINAL_HIT_POINT_MESSAGE.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    let is_hit_point_change =
        message == HIT_POINT_DAMAGE_MESSAGE || message == HIT_POINT_HEAL_MESSAGE;
    let _call = (is_hit_point_change && paused_players_can_be_hit()).then(MenuFreeCall::begin);
    // SAFETY: the trampoline continues into the original handler, which has this exact signature.
    unsafe {
        core::mem::transmute::<*mut c_void, HitPointMessage>(original)(
            actor, message, source, amount, extra,
        )
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original HUD update through the trampoline MinHook built"
)]
extern "system" fn hud_update_detour(hud: *mut c_void, seconds: f32) {
    let original = ORIGINAL_HUD_UPDATE.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    let _call = MenuFreeCall::begin();
    // SAFETY: the trampoline continues into the original update, which has this exact signature.
    unsafe { core::mem::transmute::<*mut c_void, HudUpdate>(original)(hud, seconds) }
}
