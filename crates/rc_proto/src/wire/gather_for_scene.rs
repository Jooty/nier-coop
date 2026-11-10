use serde::{Deserialize, Serialize};

use crate::wire::{MAX_POSITION_ABS, WireError};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct GatherForScene {
    pub phase: u16,
    pub position: [f32; 3],
}

impl GatherForScene {
    pub fn validate(&self) -> Result<(), WireError> {
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
