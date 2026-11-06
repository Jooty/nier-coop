use serde::{Deserialize, Serialize};

use crate::wire::{
    MAX_ANIMATION_SECONDS, MAX_HP, MAX_POSITION_ABS, MAX_YAW_ABS_RADIANS, WireError,
};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlayerState {
    pub tick: u32,
    pub phase: u16,
    pub position: [f32; 3],
    pub yaw_radians: f32,
    pub animation_id: u16,
    pub animation_seconds: f32,
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
        if !self.animation_seconds.is_finite()
            || !(0.0..=MAX_ANIMATION_SECONDS).contains(&self.animation_seconds)
        {
            return Err(WireError::OutOfRange {
                field: "animation_seconds",
            });
        }
        if self.max_hp > MAX_HP || self.hp > self.max_hp {
            return Err(WireError::OutOfRange { field: "hp" });
        }
        Ok(())
    }
}
