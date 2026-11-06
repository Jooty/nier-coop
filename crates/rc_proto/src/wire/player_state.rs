use serde::{Deserialize, Serialize};

use crate::wire::{MAX_HP, MAX_POSITION_ABS, MAX_YAW_ABS_RADIANS, MotionState, WireError};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlayerState {
    pub tick: u32,
    pub phase: u16,
    pub position: [f32; 3],
    pub yaw_radians: f32,
    pub motion: Option<MotionState>,
    pub hp: u32,
    pub max_hp: u32,
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
        if !self.yaw_radians.is_finite() || self.yaw_radians.abs() > MAX_YAW_ABS_RADIANS {
            return Err(WireError::OutOfRange { field: "yaw" });
        }
        if let Some(motion) = &self.motion {
            motion.validate()?;
        }
        if self.max_hp > MAX_HP || self.hp > self.max_hp {
            return Err(WireError::OutOfRange { field: "hp" });
        }
        Ok(())
    }
}
