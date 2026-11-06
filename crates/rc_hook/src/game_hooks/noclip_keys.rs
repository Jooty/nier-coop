use std::sync::atomic::{AtomicBool, Ordering};

use rc_game::NoclipInput;
use windows_sys::Win32::System::Threading::GetCurrentProcessId;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_CONTROL, VK_SHIFT, VK_SPACE,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

const VK_W: u16 = 0x57;
const VK_A: u16 = 0x41;
const VK_S: u16 = 0x53;
const VK_D: u16 = 0x44;

static KEYBOARD_TAKEN_BY_OVERLAY: AtomicBool = AtomicBool::new(false);

pub fn set_keyboard_taken_by_overlay(taken: bool) {
    KEYBOARD_TAKEN_BY_OVERLAY.store(taken, Ordering::Relaxed);
}

pub(super) fn read_noclip_input() -> NoclipInput {
    if KEYBOARD_TAKEN_BY_OVERLAY.load(Ordering::Relaxed) || !game_window_is_foreground() {
        return NoclipInput::default();
    }
    NoclipInput {
        forward: axis(VK_W, VK_S),
        strafe: axis(VK_D, VK_A),
        vertical: axis(VK_SPACE, VK_CONTROL),
        fast: key_down(VK_SHIFT),
    }
}

fn axis(positive: u16, negative: u16) -> i8 {
    i8::from(key_down(positive)) - i8::from(key_down(negative))
}

fn key_down(virtual_key: u16) -> bool {
    #[expect(unsafe_code, reason = "reads one key's physical state")]
    // SAFETY: GetAsyncKeyState has no preconditions.
    let state = unsafe { GetAsyncKeyState(i32::from(virtual_key)) };
    state < 0
}

fn game_window_is_foreground() -> bool {
    let mut process_id = 0_u32;
    #[expect(unsafe_code, reason = "asks which process owns the foreground window")]
    // SAFETY: `process_id` is a valid out pointer; a null window handle just yields zero.
    let owner = unsafe {
        GetWindowThreadProcessId(GetForegroundWindow(), &raw mut process_id);
        process_id
    };
    #[expect(unsafe_code, reason = "reads this process's id")]
    // SAFETY: GetCurrentProcessId has no preconditions.
    let own = unsafe { GetCurrentProcessId() };
    owner == own
}
