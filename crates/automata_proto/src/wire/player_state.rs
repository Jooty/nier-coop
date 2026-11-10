use serde::{Deserialize, Serialize};

use crate::wire::{MAX_FACING_ABS_RADIANS, MAX_POSITION_ABS, MAX_SPEED_ABS, WireError};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlayerState {
    pub tick: u32,
    pub position: [f32; 3],
    pub facing: f32,
    pub speed: f32,
    pub state_id: u16,
}

impl PlayerState {
    pub fn validate(&self) -> Result<(), WireError> {
        if self
            .position
            .iter()
            .any(|axis| !axis.is_finite() || axis.abs() > MAX_POSITION_ABS)
        {
            return Err(WireError::OutOfRange { field: "position" });
        }
        if !self.facing.is_finite() || self.facing.abs() > MAX_FACING_ABS_RADIANS {
            return Err(WireError::OutOfRange { field: "facing" });
        }
        if !self.speed.is_finite() || self.speed.abs() > MAX_SPEED_ABS {
            return Err(WireError::OutOfRange { field: "speed" });
        }
        Ok(())
    }
}
