use crate::partner::{
    CAMERA_FIELDS_A_PLAYER_BODY_UPDATE_WRITES_OFFSETS, CAMERA_FLAG_BIT_CLEARED_BY_A_MODE_REQUEST,
    CAMERA_LOCK_TARGET_HANDLE_COUNT,
};

/// What the update of a player-flagged body writes for the local player's camera.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CameraKept {
    pub lock_targets: [u32; CAMERA_LOCK_TARGET_HANDLE_COUNT],
    pub mode: u32,
    pub word_zeroed_by_a_mode_request: u32,
    pub flag_word: u32,
    pub other_fields: [u32; CAMERA_FIELDS_A_PLAYER_BODY_UPDATE_WRITES_OFFSETS.len()],
}

impl CameraKept {
    /// The flag word to write back: only the bit a mode request clears goes back to what it was.
    pub fn flag_word_put_back(&self, current: u32) -> u32 {
        (current & !CAMERA_FLAG_BIT_CLEARED_BY_A_MODE_REQUEST)
            | (self.flag_word & CAMERA_FLAG_BIT_CLEARED_BY_A_MODE_REQUEST)
    }
}
