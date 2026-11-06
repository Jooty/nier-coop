use serde::{Deserialize, Serialize};

use crate::wire::{
    MAX_SKY_AREA, MAX_SKY_BLEND_SECONDS, MAX_SKY_RECORD_BYTES, MAX_SKY_RECORD_CHUNK_BYTES,
    WireError,
};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SkyState {
    pub phase: u16,
    pub env_area: i32,
    pub env_blend_seconds: f32,
    pub wind_area: i32,
    pub weather: u8,
    pub weather_offset: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkyRecordChunk {
    pub phase: u16,
    pub area: i32,
    pub offset: u16,
    pub bytes: Vec<u8>,
}

impl SkyState {
    pub fn validate(&self) -> Result<(), WireError> {
        if !(0..=MAX_SKY_AREA).contains(&self.env_area) {
            return Err(WireError::OutOfRange { field: "env_area" });
        }
        if !(-1..=MAX_SKY_AREA).contains(&self.wind_area) {
            return Err(WireError::OutOfRange { field: "wind_area" });
        }
        if !self.env_blend_seconds.is_finite()
            || !(0.0..=MAX_SKY_BLEND_SECONDS).contains(&self.env_blend_seconds)
        {
            return Err(WireError::OutOfRange { field: "env_blend" });
        }
        Ok(())
    }
}

impl SkyRecordChunk {
    pub fn validate(&self) -> Result<(), WireError> {
        if !(0..=MAX_SKY_AREA).contains(&self.area) {
            return Err(WireError::OutOfRange { field: "area" });
        }
        if self.bytes.is_empty() || self.bytes.len() > MAX_SKY_RECORD_CHUNK_BYTES {
            return Err(WireError::OutOfRange { field: "bytes" });
        }
        if usize::from(self.offset) + self.bytes.len() > MAX_SKY_RECORD_BYTES {
            return Err(WireError::OutOfRange { field: "offset" });
        }
        Ok(())
    }
}
