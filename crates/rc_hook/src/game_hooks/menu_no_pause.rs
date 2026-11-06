use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU64, Ordering};

use rc_game::coop_pause::{PauseView, effective_control_mask};
use rc_game::marker::{EVENT_SCENE_BYTE_RVA, MAIN_MENU_STATE_RVA};
use rc_game::offsets::GET_CONTROL_MASK_FUNCTION_RVA;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::hook_slot::hook_address;

static ORIGINAL_GET_CONTROL_MASK: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static MODULE_BASE: AtomicU64 = AtomicU64::new(0);
static PARTNER_CONNECTED: AtomicBool = AtomicBool::new(false);
static APPLYING: AtomicBool = AtomicBool::new(false);

type GetControlMask = unsafe extern "system" fn(*mut c_void) -> u32;

pub fn set_menu_no_pause_wanted(partner_connected: bool) {
    PARTNER_CONNECTED.store(partner_connected, Ordering::Relaxed);
}

pub(super) fn install(build: VerifiedBuild) {
    let base = build.module_base();
    MODULE_BASE.store(base, Ordering::Relaxed);
    #[expect(unsafe_code, reason = "hooks the control mask getter by address")]
    // SAFETY: the RVA starts the game-state control mask getter (game state pointer in, mask out)
    // in the verified build, and the replacement has the same signature.
    unsafe {
        hook_address(
            "GameState_GetControlMask",
            (base + u64::from(GET_CONTROL_MASK_FUNCTION_RVA)) as usize,
            get_control_mask_detour as *const () as *mut c_void,
            &ORIGINAL_GET_CONTROL_MASK,
        );
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original getter through the trampoline MinHook built"
)]
extern "system" fn get_control_mask_detour(game_state: *mut c_void) -> u32 {
    let original = ORIGINAL_GET_CONTROL_MASK.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    // SAFETY: the trampoline continues into the original getter, which has this exact signature.
    let mask = unsafe { core::mem::transmute::<*mut c_void, GetControlMask>(original)(game_state) };
    if !PARTNER_CONNECTED.load(Ordering::Relaxed) {
        return mask;
    }
    panic::catch_unwind(|| settle(mask)).unwrap_or(mask)
}

#[expect(unsafe_code, reason = "reads two plain values in the game's image")]
fn settle(mask: u32) -> u32 {
    let base = MODULE_BASE.load(Ordering::Relaxed);
    if base == 0 {
        return mask;
    }
    // SAFETY: both addresses are fixed data in the verified build's image (the main menu state
    // dword and the event scene byte); single aligned reads cannot fault.
    let (menu_state, event_scene) = unsafe {
        (
            ptr::read_volatile((base + u64::from(MAIN_MENU_STATE_RVA)) as *const i32),
            ptr::read_volatile((base + u64::from(EVENT_SCENE_BYTE_RVA)) as *const u8),
        )
    };
    let view = PauseView {
        partner_connected: true,
        menu_open: menu_state != 0,
        event_scene: event_scene != 0,
    };
    let applies = view.applies();
    if APPLYING.swap(applies, Ordering::Relaxed) != applies {
        log_line(&format!(
            "co-op no pause: {} (mask {mask:#x} -> {:#x}, menu state {menu_state:#x})",
            if applies {
                "world keeps running under the menu"
            } else {
                "menu closed, normal mask"
            },
            effective_control_mask(mask, view),
        ));
    }
    effective_control_mask(mask, view)
}
