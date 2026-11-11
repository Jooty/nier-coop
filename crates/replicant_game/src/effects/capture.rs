use crate::game::GameError;
use crate::memory::GameMemory;
use crate::offsets::{
    EFFECT_ATTACH_BONE_OFFSET, EFFECT_ATTACH_MODE_OFFSET, EFFECT_ATTACH_SECOND_BONE_OFFSET,
    EFFECT_ATTACH_SECOND_MODE_OFFSET, EFFECT_BONE_NAME_READ_LEN, EFFECT_NAME_READ_LEN,
    EFFECT_PARAMS_NAME_OFFSET, EFFECT_PARAMS_PLAYER_ONLY_FLAG_OFFSET,
    EFFECT_PARAMS_SHAKE_STRENGTH_OFFSET, SOUND_NAME_READ_LEN,
};
use replicant_proto::{
    EFFECT_ATTACH_BYTES, EFFECT_PARAM_BYTES, EffectPlay, WORLD_EFFECT_TRANSFORM_FLOATS,
};

pub const WORLD_EFFECT_SENT_PREFIX: &[u8] = b"efP_black";

const EFFECT_NAME_FIELD: usize = EFFECT_PARAMS_NAME_OFFSET as usize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedEffect {
    pub effect: String,
    pub params: [u8; EFFECT_PARAM_BYTES],
    pub attach: [u8; EFFECT_ATTACH_BYTES],
    pub bone: String,
    pub second_bone: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectCall {
    pub bytes: Vec<u8>,
    pub attach_offset: usize,
}

impl CapturedEffect {
    pub fn attach_modes(&self) -> (u8, u8) {
        (
            self.attach[EFFECT_ATTACH_MODE_OFFSET],
            self.attach[EFFECT_ATTACH_SECOND_MODE_OFFSET],
        )
    }

    pub fn to_wire(&self, handle: u32) -> EffectPlay {
        EffectPlay {
            handle,
            effect: self.effect.clone(),
            params: self.params.to_vec(),
            attach: self.attach.to_vec(),
            bone: self.bone.clone(),
            second_bone: self.second_bone.clone(),
        }
    }
}

/// Reads the two blocks the game's play-effect call takes: the effect parameters and the attach
/// descriptor, each with self-relative name offsets. The offsets, and the fields that touch the
/// caster's own state, are cleared so the copies can be replayed on another actor.
pub fn read_effect<M: GameMemory>(
    memory: &M,
    params_address: u64,
    attach_address: u64,
) -> Result<Option<CapturedEffect>, GameError> {
    let mut params = memory.read_array::<EFFECT_PARAM_BYTES>(params_address)?;
    let mut attach = memory.read_array::<EFFECT_ATTACH_BYTES>(attach_address)?;
    let name_offset = take_offset(&mut params, EFFECT_NAME_FIELD);
    if name_offset == 0 {
        return Ok(None);
    }
    let name_address = params_address
        .wrapping_add(EFFECT_PARAMS_NAME_OFFSET)
        .wrapping_add_signed(i64::from(name_offset));
    let Some(effect) = read_text(memory, name_address, EFFECT_NAME_READ_LEN)? else {
        return Ok(None);
    };
    let bone_offset = take_offset(&mut attach, EFFECT_ATTACH_BONE_OFFSET);
    let second_offset = take_offset(&mut attach, EFFECT_ATTACH_SECOND_BONE_OFFSET);
    let bone = read_relative_text(
        memory,
        attach_address,
        EFFECT_ATTACH_BONE_OFFSET,
        bone_offset,
    )?;
    let second_bone = read_relative_text(
        memory,
        attach_address,
        EFFECT_ATTACH_SECOND_BONE_OFFSET,
        second_offset,
    )?;
    params[EFFECT_PARAMS_SHAKE_STRENGTH_OFFSET..EFFECT_PARAMS_SHAKE_STRENGTH_OFFSET + 4].fill(0);
    params[EFFECT_PARAMS_PLAYER_ONLY_FLAG_OFFSET] = 0;
    Ok(Some(CapturedEffect {
        effect,
        params,
        attach,
        bone,
        second_bone,
    }))
}

pub fn read_sound<M: GameMemory>(
    memory: &M,
    name_address: u64,
) -> Result<Option<String>, GameError> {
    if name_address == 0 {
        return Ok(None);
    }
    read_text(memory, name_address, SOUND_NAME_READ_LEN)
}

/// The name and world transform of an effect spawned by name, when the name is one of the
/// player's dark magic effects (`efP_black...`) or `any_name` is set (a hit's own sparks);
/// anything else is None after a short read.
pub fn read_world_effect<M: GameMemory>(
    memory: &M,
    name_address: u64,
    transform_address: u64,
    any_name: bool,
) -> Result<Option<(String, [f32; WORLD_EFFECT_TRANSFORM_FLOATS])>, GameError> {
    if name_address == 0 || transform_address == 0 {
        return Ok(None);
    }
    let mut prefix = [0_u8; WORLD_EFFECT_SENT_PREFIX.len()];
    memory.read(name_address, &mut prefix)?;
    if !any_name && prefix != WORLD_EFFECT_SENT_PREFIX {
        return Ok(None);
    }
    let Some(name) = read_text(memory, name_address, EFFECT_NAME_READ_LEN)? else {
        return Ok(None);
    };
    let bytes = memory.read_array::<{ WORLD_EFFECT_TRANSFORM_FLOATS * 4 }>(transform_address)?;
    let (floats, _) = bytes.as_chunks::<4>();
    let mut transform = [0.0_f32; WORLD_EFFECT_TRANSFORM_FLOATS];
    for (value, chunk) in transform.iter_mut().zip(floats) {
        *value = f32::from_le_bytes(*chunk);
    }
    Ok(Some((name, transform)))
}

/// One buffer: parameters at 0, the attach descriptor, then the names, with each self-relative
/// offset pointing at its name. The game keeps pointers into it while the effect lives.
pub fn build_effect_call(effect: &EffectPlay) -> EffectCall {
    let mut bytes = effect.params.clone();
    bytes.resize(EFFECT_PARAM_BYTES, 0);
    let attach_offset = bytes.len();
    bytes.extend_from_slice(&effect.attach);
    bytes.resize(attach_offset + EFFECT_ATTACH_BYTES, 0);
    let name = append_name(&mut bytes, &effect.effect);
    put_offset(&mut bytes, EFFECT_NAME_FIELD, name);
    for (field, text) in [
        (EFFECT_ATTACH_BONE_OFFSET, &effect.bone),
        (EFFECT_ATTACH_SECOND_BONE_OFFSET, &effect.second_bone),
    ] {
        let absolute = attach_offset + field;
        if text.is_empty() {
            bytes[absolute..absolute + 4].fill(0);
        } else {
            let position = append_name(&mut bytes, text);
            put_offset(&mut bytes, absolute, position);
        }
    }
    EffectCall {
        bytes,
        attach_offset,
    }
}

fn append_name(bytes: &mut Vec<u8>, text: &str) -> usize {
    let position = bytes.len();
    bytes.extend_from_slice(text.as_bytes());
    bytes.push(0);
    position
}

fn put_offset(bytes: &mut [u8], field: usize, target: usize) {
    let distance = (target - field) as i32;
    bytes[field..field + 4].copy_from_slice(&distance.to_le_bytes());
}

fn take_offset(block: &mut [u8], field: usize) -> i32 {
    let offset = i32::from_le_bytes([
        block[field],
        block[field + 1],
        block[field + 2],
        block[field + 3],
    ]);
    block[field..field + 4].fill(0);
    offset
}

fn read_relative_text<M: GameMemory>(
    memory: &M,
    block_address: u64,
    field: usize,
    offset: i32,
) -> Result<String, GameError> {
    if offset == 0 {
        return Ok(String::new());
    }
    let address = (block_address + field as u64).wrapping_add_signed(i64::from(offset));
    Ok(read_text(memory, address, EFFECT_BONE_NAME_READ_LEN)?.unwrap_or_default())
}

pub(super) fn read_text<M: GameMemory>(
    memory: &M,
    address: u64,
    max_bytes: usize,
) -> Result<Option<String>, GameError> {
    let mut bytes = Vec::new();
    for offset in 0..max_bytes as u64 {
        match memory.read_u8(address + offset)? {
            0 => break,
            byte => bytes.push(byte),
        }
        if bytes.len() == max_bytes {
            return Ok(None);
        }
    }
    if bytes.is_empty() {
        return Ok(None);
    }
    Ok(String::from_utf8(bytes).ok())
}
