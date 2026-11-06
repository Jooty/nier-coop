use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};

use hudhook::imgui::{Io, MouseButton};
use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::Graphics::Gdi::ScreenToClient;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_LBUTTON, VK_OEM_3, VK_RBUTTON,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{ClipCursor, GetCursorPos};

const OFF_SCREEN: [f32; 2] = [-f32::MAX, -f32::MAX];

static GAME_WINDOW: AtomicUsize = AtomicUsize::new(0);

pub(super) fn remember_game_window(window: usize) {
    GAME_WINDOW.store(window, Ordering::Relaxed);
}

pub(super) fn cursor_key_held() -> bool {
    key_down(VK_OEM_3)
}

pub(super) fn take_mouse(io: &mut Io) {
    io.mouse_draw_cursor = true;
    release_cursor_clip();
    if let Some(position) = client_cursor_position() {
        io.add_mouse_pos_event(position);
    }
    io.add_mouse_button_event(MouseButton::Left, key_down(VK_LBUTTON));
    io.add_mouse_button_event(MouseButton::Right, key_down(VK_RBUTTON));
}

pub(super) fn give_mouse_back(io: &mut Io) {
    if !io.mouse_draw_cursor {
        return;
    }
    io.mouse_draw_cursor = false;
    io.add_mouse_button_event(MouseButton::Left, false);
    io.add_mouse_button_event(MouseButton::Right, false);
    io.add_mouse_pos_event(OFF_SCREEN);
}

fn key_down(virtual_key: u16) -> bool {
    #[expect(unsafe_code, reason = "reads one key's physical state")]
    // SAFETY: GetAsyncKeyState has no preconditions.
    let state = unsafe { GetAsyncKeyState(i32::from(virtual_key)) };
    state < 0
}

fn release_cursor_clip() {
    #[expect(unsafe_code, reason = "a null rectangle removes the cursor clip")]
    // SAFETY: a null RECT pointer is the documented way to unclip.
    unsafe {
        ClipCursor(std::ptr::null())
    };
}

fn client_cursor_position() -> Option<[f32; 2]> {
    let window = GAME_WINDOW.load(Ordering::Relaxed);
    if window == 0 {
        return None;
    }
    let mut point = POINT { x: 0, y: 0 };
    #[expect(
        unsafe_code,
        reason = "reads the cursor and converts it to client space"
    )]
    // SAFETY: `point` is a valid out pointer and the handle is the game window seen in its window procedure.
    let converted = unsafe {
        GetCursorPos(&raw mut point) != 0
            && ScreenToClient(
                std::ptr::with_exposed_provenance_mut::<c_void>(window),
                &raw mut point,
            ) != 0
    };
    converted.then_some([point.x as f32, point.y as f32])
}
