use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::atomic::{AtomicPtr, AtomicU64, Ordering};

use replicant_game::offsets::{FOCUS_LOSS_PAUSE_MENU_REQUEST_RVA, UI_MANAGER_UPDATE_FUNCTION_RVA};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use nc_hook::detour::hook_address;

static ORIGINAL_UI_MANAGER_UPDATE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static MODULE_BASE: AtomicU64 = AtomicU64::new(0);
static DROPPED_REQUESTS: AtomicU64 = AtomicU64::new(0);

type UiManagerUpdate = unsafe extern "system" fn(*mut c_void, *mut c_void);

pub(super) fn install(build: VerifiedBuild) {
    let base = build.module_base();
    MODULE_BASE.store(base, Ordering::Relaxed);
    #[expect(unsafe_code, reason = "hooks the UI manager update by address")]
    // SAFETY: the RVA starts the UI manager's per-frame update (manager, frame info; no result)
    // in the verified build, and the replacement has the same signature.
    let hooked = unsafe {
        hook_address(
            "UIManager_Update",
            (base + u64::from(UI_MANAGER_UPDATE_FUNCTION_RVA)) as usize,
            ui_manager_update_detour as *const () as *mut c_void,
            &ORIGINAL_UI_MANAGER_UPDATE,
        )
    };
    if hooked {
        log_line("focus: dev build, losing window focus will not open the pause menu");
    }
}

#[expect(
    unsafe_code,
    reason = "clears a game byte and calls the original update through the MinHook trampoline"
)]
extern "system" fn ui_manager_update_detour(manager: *mut c_void, frame: *mut c_void) {
    let original = ORIGINAL_UI_MANAGER_UPDATE.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    if panic::catch_unwind(drop_focus_loss_menu_request).is_err() {
        log_line("focus: panic while dropping the focus-loss menu request");
    }
    // SAFETY: the trampoline continues into the original update, which has this exact signature.
    unsafe { core::mem::transmute::<*mut c_void, UiManagerUpdate>(original)(manager, frame) }
}

#[expect(
    unsafe_code,
    reason = "reads and clears one byte of the game's writable data"
)]
fn drop_focus_loss_menu_request() {
    let base = MODULE_BASE.load(Ordering::Relaxed);
    if base == 0 {
        return;
    }
    let request = (base + u64::from(FOCUS_LOSS_PAUSE_MENU_REQUEST_RVA)) as *mut u8;
    // SAFETY: the byte is fixed, writable game data in the verified build; this runs on the game
    // thread right before the only reader, which clears it the same way after opening the menu.
    unsafe {
        if ptr::read_volatile(request) == 0 {
            return;
        }
        ptr::write_volatile(request, 0);
    }
    let dropped = DROPPED_REQUESTS.fetch_add(1, Ordering::Relaxed) + 1;
    log_line(&format!(
        "focus: dropped the game's pause menu request from losing focus ({dropped} so far)"
    ));
}
