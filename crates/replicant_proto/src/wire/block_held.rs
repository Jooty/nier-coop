use serde::{Deserialize, Serialize};

use crate::wire::{MAX_BLOCK_ID, WireError};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockHeld {
    pub phase: u16,
    pub id: u16,
    pub held: bool,
}

impl BlockHeld {
    pub fn validate(&self) -> Result<(), WireError> {
        if self.id > MAX_BLOCK_ID {
            return Err(WireError::OutOfRange { field: "id" });
        }
        Ok(())
    }
}
