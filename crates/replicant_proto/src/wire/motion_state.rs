use serde::{Deserialize, Serialize};

use crate::wire::{MAX_MOTION_FRAME, MAX_MOTION_SPEED_ABS, WireError};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MotionState {
    pub index: u16,
    pub frame: f32,
    pub speed: f32,
    pub looping: bool,
    pub name_hash: u32,
}

impl MotionState {
    pub fn validate(&self) -> Result<(), WireError> {
        if !self.frame.is_finite() || !(0.0..=MAX_MOTION_FRAME).contains(&self.frame) {
            return Err(WireError::OutOfRange {
                field: "motion_frame",
            });
        }
        if !self.speed.is_finite() || self.speed.abs() > MAX_MOTION_SPEED_ABS {
            return Err(WireError::OutOfRange {
                field: "motion_speed",
            });
        }
        Ok(())
    }
}
