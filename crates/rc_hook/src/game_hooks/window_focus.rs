use core::ffi::c_void;
use core::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, Ordering};
use std::sync::{LazyLock, Mutex, PoisonError};

use rc_game::focus::{ActivateKind, FocusLossTracker, WM_ACTIVATE, classify_activate};
use rc_game::offsets::WINDOW_PROCEDURE_FUNCTION_RVA;
use windows_sys::Win32::UI::WindowsAndMessaging::{KillTimer, SetTimer, WM_TIMER};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::cursor_calls::clear_cursor_clip;
use crate::game_hooks::hook_slot::hook_address;
use crate::net::net_clock_ms;

const WA_ACTIVE: usize = 1;
const WA_INACTIVE: usize = 0;
const FOCUS_CLEAR_TIMER_ID: usize = 0x5243;
const FOCUS_CLEAR_TIMER_MS: u32 = 1100;
const FOCUS_CLEAR_RETRY_TIMER_MS: u32 = 100;

static ORIGINAL_WINDOW_PROCEDURE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static KEEP_RUNNING: AtomicBool = AtomicBool::new(false);
static WINDOW_UNFOCUSED: AtomicBool = AtomicBool::new(false);
static FOCUS_LOSSES: AtomicU32 = AtomicU32::new(0);
static KEY_CLEARS: AtomicU32 = AtomicU32::new(0);
static FOCUS_LOSS_TRACKER: LazyLock<Mutex<FocusLossTracker>> = LazyLock::new(Mutex::default);

type WindowProcedure = unsafe extern "system" fn(*mut c_void, u32, usize, isize) -> isize;

pub fn window_unfocused() -> bool {
    WINDOW_UNFOCUSED.load(Ordering::Relaxed)
}

pub fn take_focus_event_counts() -> (u32, u32) {
    (
        FOCUS_LOSSES.swap(0, Ordering::Relaxed),
        KEY_CLEARS.swap(0, Ordering::Relaxed),
    )
}

pub fn keep_running_enabled() -> bool {
    KEEP_RUNNING.load(Ordering::Relaxed)
}

pub fn set_keep_running(enabled: bool) {
    KEEP_RUNNING.store(enabled, Ordering::Relaxed);
}

pub(super) fn install(build: VerifiedBuild, keep_running: bool) {
    set_keep_running(keep_running);
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
        ActivateKind::Gained => {
            WINDOW_UNFOCUSED.store(false, Ordering::Relaxed);
            if keep_running {
                tracker().regain();
                #[expect(unsafe_code, reason = "stops the delayed key clear this window armed")]
                // SAFETY: KillTimer with this window and our own timer id has no other preconditions.
                unsafe {
                    KillTimer(window, FOCUS_CLEAR_TIMER_ID)
                };
            }
        }
        ActivateKind::Lost => {
            FOCUS_LOSSES.fetch_add(1, Ordering::Relaxed);
            if keep_running {
                WINDOW_UNFOCUSED.store(true, Ordering::Relaxed);
                let already_pending = {
                    let mut tracker = tracker();
                    let pending = tracker.is_pending();
                    tracker.lose(net_clock_ms());
                    pending
                };
                if !already_pending {
                    arm_clear_timer(window, FOCUS_CLEAR_TIMER_MS);
                }
                clear_cursor_clip();
                log_line("focus: window lost focus, game keeps running, held keys stay for now");
                return 0;
            }
            KEY_CLEARS.fetch_add(1, Ordering::Relaxed);
        }
        ActivateKind::NotActivation
            if message == WM_TIMER && wparam == FOCUS_CLEAR_TIMER_ID && keep_running =>
        {
            #[expect(unsafe_code, reason = "stops the one-shot delayed key clear timer")]
            // SAFETY: KillTimer with this window and our own timer id has no other preconditions.
            unsafe {
                KillTimer(window, FOCUS_CLEAR_TIMER_ID)
            };
            let (due, pending) = {
                let mut tracker = tracker();
                (tracker.take_due(net_clock_ms()), tracker.is_pending())
            };
            if due {
                // The deactivate path clears held keys; the activate call then undoes the mute and
                // the pause flag, so the game never sees itself as unfocused.
                call_original(WM_ACTIVATE, WA_INACTIVE, 0);
                call_original(WM_ACTIVATE, WA_ACTIVE, 0);
                KEY_CLEARS.fetch_add(1, Ordering::Relaxed);
                log_line("focus: focus stayed lost for over a second, held keys cleared");
            } else if pending {
                arm_clear_timer(window, FOCUS_CLEAR_RETRY_TIMER_MS);
            }
            return 0;
        }
        ActivateKind::NotActivation => {}
    }
    call_original(message, wparam, lparam)
}

fn tracker() -> std::sync::MutexGuard<'static, FocusLossTracker> {
    FOCUS_LOSS_TRACKER
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

fn arm_clear_timer(window: *mut c_void, milliseconds: u32) {
    #[expect(unsafe_code, reason = "arms the one-shot delayed key clear timer")]
    // SAFETY: SetTimer with this window and a null callback posts WM_TIMER to its procedure.
    unsafe {
        SetTimer(window, FOCUS_CLEAR_TIMER_ID, milliseconds, None)
    };
}
