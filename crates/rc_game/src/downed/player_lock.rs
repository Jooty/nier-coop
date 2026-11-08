pub const PLAYER_CONTROL_DISABLE_NATIVE_RVAS: [u32; 7] = [
    0x6E_08E0, 0x6E_0900, 0x6E_0920, 0x6E_0960, 0x6E_09A0, 0x6E_0A40, 0x6E_0A60,
];

pub const PLAYER_CONTROL_ENABLE_NATIVE_RVAS: [u32; 7] = [
    0x6E_08D0, 0x6E_08F0, 0x6E_0910, 0x6E_0950, 0x6E_0990, 0x6E_0A30, 0x6E_0A50,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerLockCall {
    Disable,
    Enable,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PlayerLock {
    disabled: bool,
}

impl PlayerLockCall {
    pub fn native_rvas(self) -> &'static [u32] {
        match self {
            Self::Disable => &PLAYER_CONTROL_DISABLE_NATIVE_RVAS,
            Self::Enable => &PLAYER_CONTROL_ENABLE_NATIVE_RVAS,
        }
    }
}

impl PlayerLock {
    pub const fn new() -> Self {
        Self { disabled: false }
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    pub fn step(&mut self, wanted: bool, can_call: bool) -> Option<PlayerLockCall> {
        if !can_call || wanted == self.disabled {
            return None;
        }
        self.disabled = wanted;
        Some(if wanted {
            PlayerLockCall::Disable
        } else {
            PlayerLockCall::Enable
        })
    }
}
