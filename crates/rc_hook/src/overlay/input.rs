use std::ffi::c_void;
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

use hudhook::imgui::{Io, MouseButton};
use windows_sys::Win32::Foundation::{POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::ScreenToClient;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_OEM_3};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CURSOR_SHOWING, CURSORINFO, GetClientRect, GetCursorInfo,
};

use crate::game_hooks;
use crate::overlay::click_sources::ClickSources;
use crate::overlay::mouse_panel::MouseDiagnostics;

const OFF_SCREEN: [f32; 2] = [-f32::MAX, -f32::MAX];

static LAST_SENT_BUTTON_BITS: AtomicU8 = AtomicU8::new(0);
static GAME_WINDOW: AtomicUsize = AtomicUsize::new(0);

pub(super) fn remember_game_window(window: usize) {
    GAME_WINDOW.store(window, Ordering::Relaxed);
}

pub(super) fn cursor_key_held() -> bool {
    key_down(VK_OEM_3)
}

pub(super) fn feed_real_mouse(io: &mut Io, cursor_mode: bool, clicks: &ClickSources) {
    let display_size = io.display_size;
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
    if let Some(position) = screen_cursor_point()
        .and_then(client_position)
        .map(|client| scale_to_display(client, display_size))
    {
        io.add_mouse_pos_event(position);
    }
    send_buttons(io, clicks.combined());
}

pub(super) fn hide_pointer(io: &mut Io) {
    io.mouse_draw_cursor = false;
    park_pointer(io);
}

pub(super) fn read_diagnostics(
    io: &Io,
    cursor_mode: bool,
    clicks: ClickSources,
) -> MouseDiagnostics {
    let screen = screen_cursor_point();
    MouseDiagnostics {
        os_cursor_screen: screen.map(|point| [point.x, point.y]),
        os_cursor_client: screen.and_then(client_position),
        client_size: client_size(),
        display_size: io.display_size,
        os_cursor_visible: os_cursor_visible(),
        imgui_position: io.mouse_pos,
        imgui_buttons: [io.mouse_down[0], io.mouse_down[1]],
        want_capture_mouse: io.want_capture_mouse,
        cursor_mode,
        game_mouse_acquired: game_hooks::game_mouse_acquired(),
        blocked_set_cursor_pos: game_hooks::blocked_set_cursor_pos_calls(),
        blocked_clip_cursor: game_hooks::blocked_clip_cursor_calls(),
        blocked_acquire: game_hooks::blocked_acquire_calls(),
        spoofed_get_cursor_pos: game_hooks::spoofed_get_cursor_pos_calls(),
        clicks,
    }
}

fn send_buttons(io: &mut Io, buttons: [bool; 2]) {
    let bits = u8::from(buttons[0]) | (u8::from(buttons[1]) << 1);
    let previous = LAST_SENT_BUTTON_BITS.swap(bits, Ordering::Relaxed);
    if previous & 1 != bits & 1 {
        io.add_mouse_button_event(MouseButton::Left, buttons[0]);
    }
    if previous & 2 != bits & 2 {
        io.add_mouse_button_event(MouseButton::Right, buttons[1]);
    }
}

fn park_pointer(io: &mut Io) {
    send_buttons(io, [false, false]);
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
    game_hooks::real_cursor_position().map(|[x, y]| POINT { x, y })
}

fn client_size() -> Option<[f32; 2]> {
    let window = GAME_WINDOW.load(Ordering::Relaxed);
    if window == 0 {
        return None;
    }
    let mut rectangle = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    #[expect(unsafe_code, reason = "reads the game window's client size")]
    // SAFETY: `rectangle` is a valid out pointer and the handle is the game window.
    let read = unsafe {
        GetClientRect(
            std::ptr::with_exposed_provenance_mut::<c_void>(window),
            &raw mut rectangle,
        ) != 0
    };
    read.then_some([
        (rectangle.right - rectangle.left) as f32,
        (rectangle.bottom - rectangle.top) as f32,
    ])
}

fn scale_to_display(client: [f32; 2], display_size: [f32; 2]) -> [f32; 2] {
    match client_size() {
        Some([width, height]) if width > 0.0 && height > 0.0 => [
            client[0] * display_size[0] / width,
            client[1] * display_size[1] / height,
        ],
        _ => client,
    }
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
