use crate::partner::{BODY_UPDATE_INPUT_MODE_COPY_LEN, PAD_INPUT_MODE};

const LOAD_EAX_FROM_A_GLOBAL: [u8; 2] = [0x8B, 0x05];
const LOAD_LEN: usize = 6;
const STORE_EAX_IN_THE_KEYBOARD_ENABLE: [u8; 6] = [0x89, 0x87, 0xC8, 0x13, 0x00, 0x00];
const STORE_EAX_IN_THE_MOUSE_ENABLE: [u8; 6] = [0x89, 0x87, 0xC4, 0x13, 0x00, 0x00];

/// The keyboard-and-mouse mode as it was before the held body's update, which sees pad mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputModeKept {
    mode: u32,
}

impl InputModeKept {
    /// Some only for the update of the held body while the game is in keyboard-and-mouse mode.
    pub fn for_body_update(body: usize, held_body: Option<usize>, mode: u32) -> Option<Self> {
        (held_body == Some(body) && mode != PAD_INPUT_MODE).then_some(Self { mode })
    }

    pub const fn mode_during_the_update(self) -> u32 {
        PAD_INPUT_MODE
    }

    /// What to write after the update; None when the game wrote a mode of its own meanwhile.
    pub fn mode_to_put_back(self, mode_after_the_update: u32) -> Option<u32> {
        (mode_after_the_update == PAD_INPUT_MODE).then_some(self.mode)
    }
}

/// True when the bytes copy the input mode global into the body's keyboard and mouse enables.
pub fn body_update_copies_the_input_mode(
    found: &[u8; BODY_UPDATE_INPUT_MODE_COPY_LEN],
    copy_rva: u32,
    input_mode_rva: u32,
) -> bool {
    let displacement = i32::from_le_bytes([found[2], found[3], found[4], found[5]]);
    let loaded_rva = copy_rva
        .checked_add(LOAD_LEN as u32)
        .and_then(|after_load| after_load.checked_add_signed(displacement));
    found[0..2] == LOAD_EAX_FROM_A_GLOBAL
        && loaded_rva == Some(input_mode_rva)
        && found[6..12] == STORE_EAX_IN_THE_KEYBOARD_ENABLE
        && found[12..18] == STORE_EAX_IN_THE_MOUSE_ENABLE
}
