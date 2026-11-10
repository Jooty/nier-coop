use serde::{Deserialize, Serialize};

use crate::wire::{MAX_PHASE_POINT, MAX_POSITION_ABS, WireError};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FollowPhase {
    pub phase: u16,
    pub point: i32,
    pub in_side_view: bool,
    pub last_normal_view: Option<[f32; 3]>,
}

impl FollowPhase {
    pub const fn at(phase: u16, point: i32) -> Self {
        Self {
            phase,
            point,
            in_side_view: false,
            last_normal_view: None,
        }
    }

    pub fn validate(&self) -> Result<(), WireError> {
        if !(0..=MAX_PHASE_POINT).contains(&self.point) {
            return Err(WireError::OutOfRange { field: "point" });
        }
        if let Some(spot) = self.last_normal_view
            && spot
                .iter()
                .any(|axis| !axis.is_finite() || axis.abs() > MAX_POSITION_ABS)
        {
            return Err(WireError::OutOfRange {
                field: "last normal view",
            });
        }
        Ok(())
    }
}
