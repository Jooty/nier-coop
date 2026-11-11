use serde::{Deserialize, Serialize};

use crate::wire::{MAX_YAW_ABS_RADIANS, MotionState, WireError};

pub const MAX_RIDDEN_BOAR_OFFSET_ABS: f32 = 64.0;

/// The boar the sender rides: where it stands in the sender's own frame (the sender's position
/// and yaw), and the body motion it plays.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RiddenBoar {
    pub offset_from_rider: [f32; 3],
    pub yaw_from_rider_radians: f32,
    pub motion: Option<MotionState>,
}

impl RiddenBoar {
    pub fn validate(&self) -> Result<(), WireError> {
        let outside = self
            .offset_from_rider
            .iter()
            .any(|axis| !axis.is_finite() || axis.abs() > MAX_RIDDEN_BOAR_OFFSET_ABS);
        if outside {
            return Err(WireError::OutOfRange {
                field: "ridden boar offset",
            });
        }
        if !self.yaw_from_rider_radians.is_finite()
            || self.yaw_from_rider_radians.abs() > MAX_YAW_ABS_RADIANS
        {
            return Err(WireError::OutOfRange {
                field: "ridden boar yaw",
            });
        }
        if let Some(motion) = &self.motion {
            motion.validate()?;
        }
        Ok(())
    }
}
