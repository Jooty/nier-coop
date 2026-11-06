use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering};

use hudhook::imgui::{Io, MouseButton};
use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::Graphics::Gdi::ScreenToClient;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, MAPVK_VSC_TO_VK_EX, MapVirtualKeyW, VK_OEM_3, VK_OEM_5, VK_OEM_8,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{CURSOR_SHOWING, CURSORINFO, GetCursorInfo};

use crate::diagnostics::log_line;
use crate::game_hooks;
use crate::overlay::click_sources::ClickSources;
use crate::overlay::mouse_panel::MouseDiagnostics;
use crate::overlay::swapchain_window;

const OFF_SCREEN: [f32; 2] = [-f32::MAX, -f32::MAX];
const FIXED_CURSOR_KEYS: [u16; 3] = [VK_OEM_3, VK_OEM_5, VK_OEM_8];
const SCAN_CODE_LEFT_OF_ONE: u32 = 0x29;
const NO_KEY: u32 = 0;

static LAST_SENT_BUTTON_BITS: AtomicU8 = AtomicU8::new(0);
static CURSOR_KEY_DOWN_LAST_FRAME: AtomicBool = AtomicBool::new(false);
static LAST_CURSOR_KEY_SEEN: AtomicU32 = AtomicU32::new(NO_KEY);
static CURSOR_MODE_LATCHED: AtomicBool = AtomicBool::new(false);

pub fn game_window() -> usize {
    swapchain_window::current().map_or(0, |window| window.expose_provenance())
}

pub(super) fn toggle_cursor_latch() {
    CURSOR_MODE_LATCHED.fetch_xor(true, Ordering::Relaxed);
}

pub(super) fn cursor_mode_requested() -> bool {
    CURSOR_MODE_LATCHED.load(Ordering::Relaxed) || cursor_key_held()
}

fn cursor_key_held() -> bool {
    let layout_key = scan_code_key();
    let held = FIXED_CURSOR_KEYS
        .iter()
        .copied()
        .chain(layout_key)
        .find(|key| key_down(*key));
    let was_down = CURSOR_KEY_DOWN_LAST_FRAME.swap(held.is_some(), Ordering::Relaxed);
    if let Some(key) = held {
        LAST_CURSOR_KEY_SEEN.store(u32::from(key), Ordering::Relaxed);
        if !was_down {
            log_line(&format!("overlay: cursor key down, VK {key:#x}"));
        }
    }
    held.is_some()
}

fn scan_code_key() -> Option<u16> {
    #[expect(
        unsafe_code,
        reason = "maps the key left of 1 to this layout's virtual key"
    )]
    // SAFETY: MapVirtualKeyW has no preconditions.
    let key = unsafe { MapVirtualKeyW(SCAN_CODE_LEFT_OF_ONE, MAPVK_VSC_TO_VK_EX) };
    u16::try_from(key).ok().filter(|key| *key != 0)
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
    let client = screen_cursor_point()
        .and_then(client_position)
        .or_else(swapchain_window::last_message_position);
    match client {
        Some(client) => io.add_mouse_pos_event(scale_to_display(client, display_size)),
        None => io.add_mouse_pos_event(OFF_SCREEN),
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
        client_size: swapchain_window::client_size(),
        game_window: swapchain_window::current_address(),
        cursor_key_vk_seen: LAST_CURSOR_KEY_SEEN.load(Ordering::Relaxed),
        cursor_mode_latched: CURSOR_MODE_LATCHED.load(Ordering::Relaxed),
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

fn scale_to_display(client: [f32; 2], display_size: [f32; 2]) -> [f32; 2] {
    match swapchain_window::client_size() {
        Some([width, height]) if width > 0.0 && height > 0.0 => [
            client[0] * display_size[0] / width,
            client[1] * display_size[1] / height,
        ],
        _ => client,
    }
}

fn client_position(mut point: POINT) -> Option<[f32; 2]> {
    let window = swapchain_window::current()?;
    #[expect(unsafe_code, reason = "converts the cursor to client space")]
    // SAFETY: `point` is a valid in-out pointer and `window` is a live window of this process.
    let converted = unsafe { ScreenToClient(window, &raw mut point) != 0 };
    converted.then_some([point.x as f32, point.y as f32])
}
