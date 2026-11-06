use serde::{Deserialize, Serialize};

use crate::wire::WireError;

pub const NO_MUSIC_TRACK: i32 = -1;
pub const MUSIC_LAYER_COUNT: usize = 4;
pub const MAX_MUSIC_POSITION_MS: u32 = 86_400_000;
pub const MAX_MUSIC_VOLUME_ABS: f32 = 100.0;
pub const MAX_MUSIC_FADE_SECONDS: f32 = 600.0;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MusicState {
    pub track: i32,
    pub position_ms: u32,
    pub track_volume: f32,
    pub master_volume: f32,
    pub layer_volumes: [f32; MUSIC_LAYER_COUNT],
    pub stopping: bool,
    pub stop_fade_seconds: f32,
}

impl MusicState {
    pub fn silent() -> Self {
        Self {
            track: NO_MUSIC_TRACK,
            position_ms: 0,
            track_volume: 1.0,
            master_volume: 1.0,
            layer_volumes: [1.0; MUSIC_LAYER_COUNT],
            stopping: false,
            stop_fade_seconds: 0.0,
        }
    }

    pub fn validate(&self) -> Result<(), WireError> {
        if self.track < NO_MUSIC_TRACK {
            return Err(WireError::OutOfRange { field: "track" });
        }
        if self.position_ms > MAX_MUSIC_POSITION_MS {
            return Err(WireError::OutOfRange {
                field: "music position",
            });
        }
        let volumes_ok = [self.track_volume, self.master_volume]
            .iter()
            .chain(&self.layer_volumes)
            .all(|volume| volume.is_finite() && volume.abs() <= MAX_MUSIC_VOLUME_ABS);
        if !volumes_ok {
            return Err(WireError::OutOfRange {
                field: "music volume",
            });
        }
        if !(0.0..=MAX_MUSIC_FADE_SECONDS).contains(&self.stop_fade_seconds) {
            return Err(WireError::OutOfRange {
                field: "music fade",
            });
        }
        Ok(())
    }
}
