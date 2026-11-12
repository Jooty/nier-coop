use serde::{Deserialize, Serialize};

use crate::wire::{MAX_POSITION_ABS, WireError};

pub const MAX_FIGHT_CAMERA_INDEX: i32 = 31;
pub const MAX_FIGHT_CAMERA_BLEND_SECONDS: f32 = 60.0;
pub const MAX_FIGHT_CAMERA_BLEND_KIND: i32 = 3;

/// The fixed camera's eye and the point it looks at, in world units.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FightCameraPose {
    pub eye: [f32; 3],
    pub look_at: [f32; 3],
}

/// The host's active camera changed in a fight window. A higher `seq` replaces a lower one;
/// `natives_before` counts the host's `_ChangeCamera` calls in the window before this switch.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FightCamera {
    pub phase: u16,
    pub seq: u32,
    pub natives_before: u32,
    pub camera: i32,
    pub blend_seconds: f32,
    pub blend_kind: i32,
    pub pose: Option<FightCameraPose>,
}

impl FightCamera {
    pub fn validate(&self) -> Result<(), WireError> {
        if !(0..=MAX_FIGHT_CAMERA_INDEX).contains(&self.camera) {
            return Err(WireError::OutOfRange { field: "camera" });
        }
        if !(0.0..=MAX_FIGHT_CAMERA_BLEND_SECONDS).contains(&self.blend_seconds) {
            return Err(WireError::OutOfRange {
                field: "blend_seconds",
            });
        }
        if !(0..=MAX_FIGHT_CAMERA_BLEND_KIND).contains(&self.blend_kind) {
            return Err(WireError::OutOfRange {
                field: "blend_kind",
            });
        }
        let pose_is_far = self.pose.is_some_and(|pose| {
            pose.eye
                .iter()
                .chain(&pose.look_at)
                .any(|axis| !axis.is_finite() || axis.abs() > MAX_POSITION_ABS)
        });
        if pose_is_far {
            return Err(WireError::OutOfRange { field: "pose" });
        }
        Ok(())
    }
}
