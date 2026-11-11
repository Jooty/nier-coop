use serde::{Deserialize, Serialize};

use crate::wire::{BULLET_STYLE_FLOATS, MAX_BULLET_STYLE_ABS, MAX_POSITION_ABS, WireError};

pub const MAX_EFFECT_NAME_BYTES: usize = 64;
pub const MAX_EFFECT_BONE_BYTES: usize = 48;
pub const EFFECT_PARAM_BYTES: usize = 0x48;
pub const EFFECT_ATTACH_BYTES: usize = 0x58;
pub const MAX_SOUND_NAME_BYTES: usize = 64;
pub const MAX_VOICE_CHARACTER_BYTES: usize = 32;
pub const WORLD_EFFECT_TRANSFORM_FLOATS: usize = 12;
pub const WORLD_EFFECT_ARGUMENT_COUNT: usize = 5;

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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceCharacter {
    pub state: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BulletEffectPlay {
    pub position: [f32; 3],
    pub effect: EffectPlay,
}

/// An effect a spell spawned by name at a world transform (3x4 rows, translation in the last
/// column), with the spawn call's remaining arguments as raw words.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldEffectPlay {
    pub effect: String,
    pub transform: [f32; WORLD_EFFECT_TRANSFORM_FLOATS],
    pub arguments: [u32; WORLD_EFFECT_ARGUMENT_COUNT],
    pub handle: u32,
    pub look: Option<EffectLook>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct EffectLook {
    pub style: [f32; BULLET_STYLE_FLOATS],
    pub extra: f32,
    pub flag: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldEffectMove {
    pub handle: u32,
    pub transform: [f32; WORLD_EFFECT_TRANSFORM_FLOATS],
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

impl VoiceCharacter {
    pub fn validate(&self) -> Result<(), WireError> {
        validate_text(
            &self.state,
            MAX_VOICE_CHARACTER_BYTES,
            "voice character",
            false,
        )
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

impl WorldEffectPlay {
    pub fn validate(&self) -> Result<(), WireError> {
        validate_text(&self.effect, MAX_EFFECT_NAME_BYTES, "effect", false)?;
        validate_world_transform(&self.transform)?;
        self.look.map_or(Ok(()), |look| look.validate())
    }
}

impl EffectLook {
    pub fn validate(&self) -> Result<(), WireError> {
        let in_range = self
            .style
            .iter()
            .chain([&self.extra])
            .all(|value| value.is_finite() && value.abs() <= MAX_BULLET_STYLE_ABS);
        if in_range {
            Ok(())
        } else {
            Err(WireError::OutOfRange {
                field: "effect look",
            })
        }
    }
}

impl WorldEffectMove {
    pub fn validate(&self) -> Result<(), WireError> {
        validate_world_transform(&self.transform)
    }
}

fn validate_world_transform(
    transform: &[f32; WORLD_EFFECT_TRANSFORM_FLOATS],
) -> Result<(), WireError> {
    let translation = [transform[3], transform[7], transform[11]];
    let finite = transform.iter().all(|value| value.is_finite());
    if !finite || translation.iter().any(|axis| axis.abs() > MAX_POSITION_ABS) {
        return Err(WireError::OutOfRange {
            field: "world effect transform",
        });
    }
    Ok(())
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
