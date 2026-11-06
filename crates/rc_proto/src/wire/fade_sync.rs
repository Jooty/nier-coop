use serde::{Deserialize, Serialize};

use crate::wire::{MAX_FADE_SECONDS, WireError};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FadeSync {
    pub fading_out: bool,
    pub kind: u8,
    pub depth: u32,
    pub seconds: f32,
}

impl FadeSync {
    pub fn validate(&self) -> Result<(), WireError> {
        if !self.seconds.is_finite() || !(0.0..=MAX_FADE_SECONDS).contains(&self.seconds) {
            return Err(WireError::OutOfRange { field: "seconds" });
        }
        if !matches!(self.kind, 1 | 2 | 5) {
            return Err(WireError::OutOfRange { field: "kind" });
        }
        Ok(())
    }
}
