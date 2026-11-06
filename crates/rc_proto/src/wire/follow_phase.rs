use serde::{Deserialize, Serialize};

use crate::wire::{MAX_PHASE_POINT, WireError};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FollowPhase {
    pub phase: u16,
    pub point: i32,
}

impl FollowPhase {
    pub fn validate(&self) -> Result<(), WireError> {
        if !(0..=MAX_PHASE_POINT).contains(&self.point) {
            return Err(WireError::OutOfRange { field: "point" });
        }
        Ok(())
    }
}
