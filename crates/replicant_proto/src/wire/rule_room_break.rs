use serde::{Deserialize, Serialize};

use crate::wire::{MAX_STORY_CALL_NAME_BYTES, WireError};

/// The guest's player broke a rule room's rule in its own game: `handler` is the break handler its
/// game fired, asked of the host so the rule breaks once there and the reset reaches both.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleRoomBreak {
    pub phase: u16,
    pub handler: String,
}

impl RuleRoomBreak {
    pub fn validate(&self) -> Result<(), WireError> {
        let bytes = self.handler.as_bytes();
        if bytes.is_empty()
            || bytes.len() > MAX_STORY_CALL_NAME_BYTES
            || !bytes.iter().all(u8::is_ascii_graphic)
        {
            return Err(WireError::OutOfRange {
                field: "rule room break handler",
            });
        }
        Ok(())
    }
}
