use automata_game::event_follow::Unattended;
use automata_game::travel::game_is_paused;
use windows_sys::Win32::System::Threading::GetCurrentProcessId;
use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

use crate::game_calls::GameCalls;
use crate::game_hooks::game_says_its_window_lost_focus;

pub(super) fn local_attendance(calls: &GameCalls) -> Option<Unattended> {
    Unattended::of(
        game_says_its_window_lost_focus(),
        game_window_is_foreground(),
        game_is_paused(calls.pause_state()),
    )
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
