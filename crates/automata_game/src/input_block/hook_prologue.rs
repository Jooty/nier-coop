use crate::address::{
    CURSOR_RECENTRE_FUNCTION, GameAddress, POLL_DEVICES_FUNCTION, POLL_MOUSE_FUNCTION,
};
use crate::input_block::{CURSOR_RECENTRE_PROLOGUE, POLL_DEVICES_PROLOGUE, POLL_MOUSE_PROLOGUE};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputHook {
    PollDevices,
    PollMouse,
    CursorRecentre,
}

impl InputHook {
    pub const ALL: [Self; 3] = [Self::PollDevices, Self::PollMouse, Self::CursorRecentre];

    pub fn label(self) -> &'static str {
        match self {
            Self::PollDevices => "input device poll",
            Self::PollMouse => "mouse poll",
            Self::CursorRecentre => "cursor recentre",
        }
    }

    pub fn address(self) -> GameAddress {
        match self {
            Self::PollDevices => POLL_DEVICES_FUNCTION,
            Self::PollMouse => POLL_MOUSE_FUNCTION,
            Self::CursorRecentre => CURSOR_RECENTRE_FUNCTION,
        }
    }

    pub fn expected_prologue(self) -> &'static [u8] {
        match self {
            Self::PollDevices => &POLL_DEVICES_PROLOGUE,
            Self::PollMouse => &POLL_MOUSE_PROLOGUE,
            Self::CursorRecentre => &CURSOR_RECENTRE_PROLOGUE,
        }
    }
}

pub fn prologue_matches(hook: InputHook, found: &[u8]) -> bool {
    found.starts_with(hook.expected_prologue())
}
