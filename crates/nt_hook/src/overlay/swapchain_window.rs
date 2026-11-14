use core::ffi::c_void;
use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT};
use windows_sys::Win32::System::Threading::GetCurrentProcessId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GW_OWNER, GetClientRect, GetWindow, GetWindowThreadProcessId, IsWindow,
    IsWindowVisible, PostMessageW, WM_CLOSE,
};

use crate::diagnostics::log_line;

const NO_MESSAGE_POSITION: i32 = i32::MIN;

static GAME_WINDOW: AtomicUsize = AtomicUsize::new(0);
static LAST_MESSAGE_X: AtomicI32 = AtomicI32::new(NO_MESSAGE_POSITION);
static LAST_MESSAGE_Y: AtomicI32 = AtomicI32::new(NO_MESSAGE_POSITION);

struct Largest {
    process: u32,
    window: usize,
    area: i64,
}

pub fn remember(window: usize) {
    if window != 0 && GAME_WINDOW.swap(window, Ordering::Relaxed) != window {
        log_line(&format!(
            "overlay: game window (swapchain output) is {window:#x}"
        ));
    }
}

pub fn remember_client_position(lparam: isize) {
    let x = i32::from((lparam & 0xFFFF) as u16 as i16);
    let y = i32::from(((lparam >> 16) & 0xFFFF) as u16 as i16);
    LAST_MESSAGE_X.store(x, Ordering::Relaxed);
    LAST_MESSAGE_Y.store(y, Ordering::Relaxed);
}

pub fn last_message_position() -> Option<[f32; 2]> {
    let x = LAST_MESSAGE_X.load(Ordering::Relaxed);
    let y = LAST_MESSAGE_Y.load(Ordering::Relaxed);
    (x != NO_MESSAGE_POSITION && y != NO_MESSAGE_POSITION).then_some([x as f32, y as f32])
}

pub fn current_address() -> usize {
    GAME_WINDOW.load(Ordering::Relaxed)
}

pub fn close_game_window() {
    let Some(window) = current() else {
        log_line("net: no game window to close");
        return;
    };
    #[expect(unsafe_code, reason = "asks the game window to close itself")]
    // SAFETY: PostMessageW accepts any window handle and posts without blocking.
    let posted = unsafe { PostMessageW(window, WM_CLOSE, 0, 0) != 0 };
    log_line(&format!(
        "net: asked the game window to close, posted {posted}"
    ));
}

pub fn current() -> Option<HWND> {
    let remembered = GAME_WINDOW.load(Ordering::Relaxed);
    if remembered != 0 && is_window(remembered) {
        return Some(as_handle(remembered));
    }
    let found = find_process_window()?;
    remember(found);
    Some(as_handle(found))
}

pub fn client_size() -> Option<[f32; 2]> {
    let window = current()?;
    let rectangle = client_rectangle(window)?;
    Some([
        (rectangle.right - rectangle.left) as f32,
        (rectangle.bottom - rectangle.top) as f32,
    ])
}

fn as_handle(window: usize) -> HWND {
    std::ptr::with_exposed_provenance_mut::<c_void>(window)
}

fn is_window(window: usize) -> bool {
    #[expect(
        unsafe_code,
        reason = "asks whether the remembered handle is still a window"
    )]
    // SAFETY: IsWindow accepts any handle value.
    unsafe {
        IsWindow(as_handle(window)) != 0
    }
}

fn client_rectangle(window: HWND) -> Option<RECT> {
    let mut rectangle = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    #[expect(unsafe_code, reason = "reads the game window's client size")]
    // SAFETY: `rectangle` is a valid out pointer.
    let read = unsafe { GetClientRect(window, &raw mut rectangle) != 0 };
    read.then_some(rectangle)
}

fn own_process_id() -> u32 {
    #[expect(unsafe_code, reason = "reads this process's id")]
    // SAFETY: GetCurrentProcessId has no preconditions.
    unsafe {
        GetCurrentProcessId()
    }
}

fn find_process_window() -> Option<usize> {
    let mut largest = Largest {
        process: own_process_id(),
        window: 0,
        area: 0,
    };
    #[expect(
        unsafe_code,
        reason = "enumerates top-level windows to find the game's"
    )]
    // SAFETY: the callback only reads through the `Largest` pointer, which outlives the call.
    unsafe {
        EnumWindows(Some(consider_window), (&raw mut largest) as LPARAM);
    }
    (largest.window != 0).then_some(largest.window)
}

#[expect(unsafe_code, reason = "window enumeration callback")]
unsafe extern "system" fn consider_window(window: HWND, parameter: LPARAM) -> i32 {
    const KEEP_ENUMERATING: i32 = 1;
    // SAFETY: `parameter` is the `Largest` that find_process_window passed.
    let largest = unsafe { &mut *(parameter as *mut Largest) };
    let mut process = 0_u32;
    // SAFETY: `process` is a valid out pointer and `window` comes from the enumeration.
    let (visible, owner) = unsafe {
        GetWindowThreadProcessId(window, &raw mut process);
        (IsWindowVisible(window) != 0, GetWindow(window, GW_OWNER))
    };
    if process != largest.process || !visible || !owner.is_null() {
        return KEEP_ENUMERATING;
    }
    if let Some(rectangle) = client_rectangle(window) {
        let area = i64::from(rectangle.right - rectangle.left)
            * i64::from(rectangle.bottom - rectangle.top);
        if area > largest.area {
            largest.area = area;
            largest.window = window.expose_provenance();
        }
    }
    KEEP_ENUMERATING
}
