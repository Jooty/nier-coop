use serde::{Deserialize, Serialize};

use crate::wire::WireError;

pub const MAX_SCRIPT_MESSAGE_NUMBER: u32 = 1_023;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptMessage {
    pub phase: u16,
    pub message: u32,
}

impl ScriptMessage {
    pub fn validate(&self) -> Result<(), WireError> {
        if self.message > MAX_SCRIPT_MESSAGE_NUMBER {
            return Err(WireError::OutOfRange { field: "message" });
        }
        Ok(())
    }
}
