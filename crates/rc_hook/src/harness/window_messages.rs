use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows_sys::Win32::Graphics::Gdi::ClientToScreen;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    PostMessageW, SetCursorPos, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE,
    WM_RBUTTONDOWN, WM_RBUTTONUP,
};

use crate::overlay::game_window;

const EXTENDED_SCAN_CODE_BIT: u8 = 0x80;
const EXTENDED_KEY_LPARAM_BIT: isize = 1 << 24;
const SCAN_CODE_LPARAM_SHIFT: u32 = 16;
const KEY_UP_LPARAM_BITS: isize = (1 << 30) | (1 << 31);
const REPEAT_COUNT_ONE: isize = 1;
const MK_LBUTTON: usize = 0x0001;
const MK_RBUTTON: usize = 0x0002;

pub(super) fn game_window_handle() -> Option<HWND> {
    match game_window() {
        0 => None,
        window => Some(window as HWND),
    }
}

pub(super) fn post_key(virtual_key: u8, direct_input_scan_code: u8, down: bool) {
    let Some(window) = game_window_handle() else {
        return;
    };
    let mut lparam = REPEAT_COUNT_ONE
        | (isize::from(direct_input_scan_code & !EXTENDED_SCAN_CODE_BIT) << SCAN_CODE_LPARAM_SHIFT);
    if direct_input_scan_code & EXTENDED_SCAN_CODE_BIT != 0 {
        lparam |= EXTENDED_KEY_LPARAM_BIT;
    }
    let message = if down {
        WM_KEYDOWN
    } else {
        lparam |= KEY_UP_LPARAM_BITS;
        WM_KEYUP
    };
    post(window, message, usize::from(virtual_key), lparam);
}

pub(super) fn post_mouse_button(index: usize, down: bool, x: i32, y: i32) {
    let Some(window) = game_window_handle() else {
        return;
    };
    let (message, held) = match (index, down) {
        (0, true) => (WM_LBUTTONDOWN, MK_LBUTTON),
        (0, false) => (WM_LBUTTONUP, 0),
        (_, true) => (WM_RBUTTONDOWN, MK_RBUTTON),
        (_, false) => (WM_RBUTTONUP, 0),
    };
    post(window, message, held, client_lparam(x, y));
}

pub(super) fn move_pointer(window: HWND, x: i32, y: i32) {
    let mut point = POINT { x, y };
    #[expect(
        unsafe_code,
        reason = "converts a client position and moves the real cursor there"
    )]
    // SAFETY: `point` is a valid in/out pointer and `window` is the game window handle.
    unsafe {
        ClientToScreen(window, &raw mut point);
        SetCursorPos(point.x, point.y);
    }
    post(window, WM_MOUSEMOVE, 0, client_lparam(x, y));
}

fn client_lparam(x: i32, y: i32) -> isize {
    (((y & 0xFFFF) << 16) | (x & 0xFFFF)) as isize
}

fn post(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) {
    #[expect(unsafe_code, reason = "posts an input message to the game window")]
    // SAFETY: PostMessageW only queues the message; a stale handle makes it fail harmlessly.
    unsafe {
        PostMessageW(window, message, wparam, lparam)
    };
}
