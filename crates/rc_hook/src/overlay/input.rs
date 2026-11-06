use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};

use hudhook::imgui::{Io, MouseButton};
use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::Graphics::Gdi::ScreenToClient;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_LBUTTON, VK_OEM_3, VK_RBUTTON,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CURSOR_SHOWING, CURSORINFO, GetCursorInfo, GetCursorPos,
};

use crate::game_hooks;
use crate::overlay::mouse_panel::MouseDiagnostics;

const OFF_SCREEN: [f32; 2] = [-f32::MAX, -f32::MAX];

static GAME_WINDOW: AtomicUsize = AtomicUsize::new(0);

pub(super) fn remember_game_window(window: usize) {
    GAME_WINDOW.store(window, Ordering::Relaxed);
}

pub(super) fn cursor_key_held() -> bool {
    key_down(VK_OEM_3)
}

pub(super) fn feed_real_mouse(io: &mut Io, cursor_mode: bool) {
    let os_cursor_visible = os_cursor_visible();
    if cursor_mode {
        game_hooks::clear_cursor_clip();
    }
    io.mouse_draw_cursor = !os_cursor_visible;
    if !cursor_mode && !os_cursor_visible {
        // a hidden OS cursor with ~ up is the game's own mouse look: keep the pointer off the windows
        park_pointer(io);
        return;
    }
    if let Some(position) = screen_cursor_point().and_then(client_position) {
        io.add_mouse_pos_event(position);
    }
    io.add_mouse_button_event(MouseButton::Left, key_down(VK_LBUTTON));
    io.add_mouse_button_event(MouseButton::Right, key_down(VK_RBUTTON));
}

pub(super) fn hide_pointer(io: &mut Io) {
    io.mouse_draw_cursor = false;
    park_pointer(io);
}

pub(super) fn read_diagnostics(io: &Io, cursor_mode: bool) -> MouseDiagnostics {
    let screen = screen_cursor_point();
    MouseDiagnostics {
        os_cursor_screen: screen.map(|point| [point.x, point.y]),
        os_cursor_client: screen.and_then(client_position),
        os_cursor_visible: os_cursor_visible(),
        imgui_position: io.mouse_pos,
        imgui_buttons: [io.mouse_down[0], io.mouse_down[1]],
        want_capture_mouse: io.want_capture_mouse,
        cursor_mode,
        game_mouse_acquired: game_hooks::game_mouse_acquired(),
        blocked_set_cursor_pos: game_hooks::blocked_set_cursor_pos_calls(),
        blocked_clip_cursor: game_hooks::blocked_clip_cursor_calls(),
        blocked_acquire: game_hooks::blocked_acquire_calls(),
    }
}

fn park_pointer(io: &mut Io) {
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

fn os_cursor_visible() -> bool {
    let mut info = CURSORINFO {
        cbSize: size_of::<CURSORINFO>() as u32,
        flags: 0,
        hCursor: std::ptr::null_mut(),
        ptScreenPos: POINT { x: 0, y: 0 },
    };
    #[expect(unsafe_code, reason = "asks whether the OS cursor is showing")]
    // SAFETY: `info` is a valid CURSORINFO with its size field set.
    let read = unsafe { GetCursorInfo(&raw mut info) != 0 };
    read && info.flags & CURSOR_SHOWING != 0
}

fn screen_cursor_point() -> Option<POINT> {
    let mut point = POINT { x: 0, y: 0 };
    #[expect(unsafe_code, reason = "reads the cursor position")]
    // SAFETY: `point` is a valid out pointer.
    let read = unsafe { GetCursorPos(&raw mut point) != 0 };
    read.then_some(point)
}

fn client_position(mut point: POINT) -> Option<[f32; 2]> {
    let window = GAME_WINDOW.load(Ordering::Relaxed);
    if window == 0 {
        return None;
    }
    #[expect(unsafe_code, reason = "converts the cursor to client space")]
    // SAFETY: `point` is a valid in-out pointer and the handle is the game window seen in its window procedure.
    let converted = unsafe {
        ScreenToClient(
            std::ptr::with_exposed_provenance_mut::<c_void>(window),
            &raw mut point,
        ) != 0
    };
    converted.then_some([point.x as f32, point.y as f32])
}
