use serde::{Deserialize, Serialize};

use crate::wire::{MAX_BLOCK_ID, MAX_POSITION_ABS, WireError};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BlockMoved {
    pub phase: u16,
    pub id: u16,
    pub position: [f32; 3],
}

impl BlockMoved {
    pub fn validate(&self) -> Result<(), WireError> {
        if self.id > MAX_BLOCK_ID {
            return Err(WireError::OutOfRange { field: "id" });
        }
        if self
            .position
            .iter()
            .any(|axis| !axis.is_finite() || axis.abs() > MAX_POSITION_ABS)
        {
            return Err(WireError::OutOfRange { field: "position" });
        }
        Ok(())
    }
}
