use serde::{Deserialize, Serialize};

use crate::wire::{MAX_POSITION_ABS, MAX_YAW_ABS_RADIANS, WireError};

pub const MAX_POWER_MODEL_INSTANCE: u16 = 4095;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerModelKind {
    BlackHand030,
    BlackHand020,
    BlackHandAlt030,
    BlackHandAlt020,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PowerModelState {
    pub kind: PowerModelKind,
    pub instance: u16,
    pub position: [f32; 3],
    pub yaw_radians: f32,
}

impl PowerModelState {
    pub fn validate(&self) -> Result<(), WireError> {
        if self.instance > MAX_POWER_MODEL_INSTANCE {
            return Err(WireError::OutOfRange {
                field: "power model instance",
            });
        }
        let outside = self
            .position
            .iter()
            .any(|axis| !axis.is_finite() || axis.abs() > MAX_POSITION_ABS);
        if outside {
            return Err(WireError::OutOfRange {
                field: "power model position",
            });
        }
        if !self.yaw_radians.is_finite() || self.yaw_radians.abs() > MAX_YAW_ABS_RADIANS {
            return Err(WireError::OutOfRange {
                field: "power model yaw",
            });
        }
        Ok(())
    }
}
