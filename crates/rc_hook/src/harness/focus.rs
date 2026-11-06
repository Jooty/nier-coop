use std::sync::atomic::{AtomicIsize, Ordering};
use std::thread;
use std::time::Duration;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallWindowProcW, GWLP_WNDPROC, PostMessageW, SetWindowLongPtrW, WM_ACTIVATE, WM_ACTIVATEAPP,
    WM_KILLFOCUS, WM_NCACTIVATE, WM_SETFOCUS,
};

use crate::diagnostics::log_line;
use crate::overlay::game_window;

const WINDOW_POLL_INTERVAL: Duration = Duration::from_millis(250);
const WINDOW_POLL_ATTEMPTS: u32 = 480;
const ACTIVE: usize = 1;
const INACTIVE_LOW_WORD_MASK: usize = 0xFFFF;
const TRUE_LRESULT: LRESULT = 1;

static PREVIOUS_WINDOW_PROC: AtomicIsize = AtomicIsize::new(0);

pub fn install_window_focus() {
    let Some(window) = wait_for_game_window() else {
        log_line("harness: game window never seen, focus keeping is off");
        return;
    };
    #[expect(
        unsafe_code,
        reason = "subclasses the game window to swallow focus loss messages"
    )]
    // SAFETY: `window` is the game's window and the replacement has the window procedure signature.
    let previous = unsafe {
        SetWindowLongPtrW(
            window,
            GWLP_WNDPROC,
            focus_keeping_window_proc as *const () as usize as isize,
        )
    };
    if previous == 0 {
        log_line("harness: cannot subclass the game window");
        return;
    }
    PREVIOUS_WINDOW_PROC.store(previous, Ordering::Release);
    #[expect(unsafe_code, reason = "tells the game it is active")]
    // SAFETY: PostMessageW only queues the messages.
    unsafe {
        PostMessageW(window, WM_ACTIVATEAPP, ACTIVE, 0);
        PostMessageW(window, WM_SETFOCUS, 0, 0);
    }
    log_line("harness: focus keeping on, the game stays active when unfocused");
}

fn wait_for_game_window() -> Option<HWND> {
    for _ in 0..WINDOW_POLL_ATTEMPTS {
        match game_window() {
            0 => thread::sleep(WINDOW_POLL_INTERVAL),
            window => return Some(window as HWND),
        }
    }
    None
}

#[expect(
    unsafe_code,
    reason = "window procedure called by the system with raw handles"
)]
unsafe extern "system" fn focus_keeping_window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let loses_focus = match message {
        WM_ACTIVATEAPP => wparam == 0,
        WM_ACTIVATE => wparam & INACTIVE_LOW_WORD_MASK == 0,
        WM_NCACTIVATE => wparam == 0,
        WM_KILLFOCUS => true,
        _ => false,
    };
    if loses_focus {
        return if message == WM_NCACTIVATE {
            TRUE_LRESULT
        } else {
            0
        };
    }
    let previous = PREVIOUS_WINDOW_PROC.load(Ordering::Acquire);
    // SAFETY: `previous` is the window procedure that was installed before ours.
    unsafe {
        CallWindowProcW(
            core::mem::transmute::<
                isize,
                Option<unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT>,
            >(previous),
            window,
            message,
            wparam,
            lparam,
        )
    }
}
