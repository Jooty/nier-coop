use core::ffi::c_void;
use core::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

use rc_game::focus::{ActivateKind, classify_activate};
use rc_game::offsets::WINDOW_PROCEDURE_FUNCTION_RVA;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::cursor_calls::clear_cursor_clip;
use crate::game_hooks::hook_slot::hook_address;

const WA_ACTIVE: usize = 1;

static ORIGINAL_WINDOW_PROCEDURE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static KEEP_RUNNING: AtomicBool = AtomicBool::new(false);
static WINDOW_UNFOCUSED: AtomicBool = AtomicBool::new(false);

type WindowProcedure = unsafe extern "system" fn(*mut c_void, u32, usize, isize) -> isize;

pub fn window_unfocused() -> bool {
    WINDOW_UNFOCUSED.load(Ordering::Relaxed)
}

pub(super) fn install(build: VerifiedBuild) {
    KEEP_RUNNING.store(true, Ordering::Relaxed);
    let address = (build.module_base() + u64::from(WINDOW_PROCEDURE_FUNCTION_RVA)) as usize;
    #[expect(
        unsafe_code,
        reason = "hooks the game's window procedure by its address"
    )]
    // SAFETY: the RVA starts the WindowClass1 procedure (hwnd, message, wparam, lparam) in the
    // verified build, and the replacement has the same signature.
    unsafe {
        hook_address(
            "window procedure",
            address,
            window_procedure_detour as *const () as *mut c_void,
            &ORIGINAL_WINDOW_PROCEDURE,
        )
    };
}

#[expect(
    unsafe_code,
    reason = "calls the original window procedure through the trampoline MinHook built"
)]
extern "system" fn window_procedure_detour(
    window: *mut c_void,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    let original = ORIGINAL_WINDOW_PROCEDURE.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    // SAFETY: the trampoline continues into the original, which has this exact signature.
    let call_original = |message: u32, wparam: usize, lparam: isize| unsafe {
        core::mem::transmute::<*mut c_void, WindowProcedure>(original)(
            window, message, wparam, lparam,
        )
    };
    let activation = classify_activate(message, wparam);
    let keep_running = KEEP_RUNNING.load(Ordering::Relaxed);
    match activation {
        ActivateKind::Gained => WINDOW_UNFOCUSED.store(false, Ordering::Relaxed),
        ActivateKind::Lost if keep_running => {
            WINDOW_UNFOCUSED.store(true, Ordering::Relaxed);
            // The deactivate path clears held keys and releases the mouse; the activate call then
            // undoes the mute and the pause flag, so the game never sees itself as unfocused.
            let result = call_original(message, wparam, lparam);
            call_original(message, WA_ACTIVE, lparam);
            clear_cursor_clip();
            log_line("focus: window lost focus, game keeps running");
            return result;
        }
        ActivateKind::Lost | ActivateKind::NotActivation => {}
    }
    call_original(message, wparam, lparam)
}
