use serde::{Deserialize, Serialize};

use crate::wire::{MAX_POSITION_ABS, WireError};

pub const MAX_EFFECT_NAME_BYTES: usize = 64;
pub const MAX_EFFECT_BONE_BYTES: usize = 48;
pub const EFFECT_PARAM_BYTES: usize = 0x48;
pub const EFFECT_ATTACH_BYTES: usize = 0x58;
pub const MAX_SOUND_NAME_BYTES: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectPlay {
    pub effect: String,
    pub params: Vec<u8>,
    pub attach: Vec<u8>,
    pub bone: String,
    pub second_bone: String,
    pub handle: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectStop {
    pub handle: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoundPlay {
    pub sound: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BulletEffectPlay {
    pub position: [f32; 3],
    pub effect: EffectPlay,
}

impl EffectPlay {
    pub fn validate(&self) -> Result<(), WireError> {
        validate_text(&self.effect, MAX_EFFECT_NAME_BYTES, "effect", false)?;
        validate_text(&self.bone, MAX_EFFECT_BONE_BYTES, "bone", true)?;
        validate_text(
            &self.second_bone,
            MAX_EFFECT_BONE_BYTES,
            "second bone",
            true,
        )?;
        if self.params.len() != EFFECT_PARAM_BYTES {
            return Err(WireError::OutOfRange {
                field: "effect params",
            });
        }
        if self.attach.len() != EFFECT_ATTACH_BYTES {
            return Err(WireError::OutOfRange {
                field: "effect attach",
            });
        }
        Ok(())
    }
}

impl SoundPlay {
    pub fn validate(&self) -> Result<(), WireError> {
        validate_text(&self.sound, MAX_SOUND_NAME_BYTES, "sound", false)
    }
}

impl BulletEffectPlay {
    pub fn validate(&self) -> Result<(), WireError> {
        let outside = self
            .position
            .iter()
            .any(|axis| !axis.is_finite() || axis.abs() > MAX_POSITION_ABS);
        if outside {
            return Err(WireError::OutOfRange {
                field: "bullet effect position",
            });
        }
        self.effect.validate()
    }
}

fn validate_text(
    text: &str,
    max_bytes: usize,
    field: &'static str,
    may_be_empty: bool,
) -> Result<(), WireError> {
    let empty_ok = may_be_empty || !text.is_empty();
    if !empty_ok || text.len() > max_bytes || text.chars().any(char::is_control) {
        return Err(WireError::OutOfRange { field });
    }
    Ok(())
}
