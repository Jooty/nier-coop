use serde::{Deserialize, Serialize};

use crate::wire::{MAX_STORY_CALL_NAME_BYTES, WireError};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedFunctionRan {
    pub phase: u16,
    pub name: String,
}

impl SharedFunctionRan {
    pub fn validate(&self) -> Result<(), WireError> {
        let bytes = self.name.as_bytes();
        if bytes.is_empty()
            || bytes.len() > MAX_STORY_CALL_NAME_BYTES
            || !bytes.iter().all(u8::is_ascii_graphic)
        {
            return Err(WireError::OutOfRange {
                field: "shared function name",
            });
        }
        Ok(())
    }
}
