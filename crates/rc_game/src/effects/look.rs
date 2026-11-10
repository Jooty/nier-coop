use std::collections::HashSet;

use rc_proto::{BULLET_STYLE_FLOATS, EffectLook};

use crate::effects::effect_instance_address;
use crate::game::GameError;
use crate::memory::GameMemory;
use crate::offsets::{
    EFFECT_INSTANCE_CHANGED_FLAG_OFFSET, EFFECT_INSTANCE_EXTRA_OFFSET, EFFECT_INSTANCE_STYLE_OFFSET,
};

pub const EFFECT_LOOK_RETRY_FRAMES: u32 = 5;
const STYLE_ALPHA_INDEX: usize = 3;
const FLOAT_BYTES: usize = 4;
const STYLE_BYTES: usize = BULLET_STYLE_FLOATS * FLOAT_BYTES;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LookAttempt {
    Use,
    Wait,
    GiveUp,
}

pub fn look_attempt(frames_waited: u32, instance_found: bool) -> LookAttempt {
    if instance_found {
        LookAttempt::Use
    } else if frames_waited >= EFFECT_LOOK_RETRY_FRAMES {
        LookAttempt::GiveUp
    } else {
        LookAttempt::Wait
    }
}

pub fn look_is_set(look: &EffectLook) -> bool {
    look.style[STYLE_ALPHA_INDEX] > 0.0
}

pub fn look_needs_write(sender: &EffectLook, receiver: &EffectLook) -> bool {
    look_is_set(sender) && look_bits(sender) != look_bits(receiver)
}

fn look_bits(look: &EffectLook) -> ([u32; BULLET_STYLE_FLOATS], u32, u8) {
    (
        look.style.map(f32::to_bits),
        look.extra.to_bits(),
        look.flag,
    )
}

pub fn read_effect_look<M: GameMemory>(
    memory: &M,
    handle: u32,
) -> Result<Option<EffectLook>, GameError> {
    let Some(instance) = effect_instance_address(memory, handle)? else {
        return Ok(None);
    };
    let bytes = memory.read_array::<STYLE_BYTES>(instance + EFFECT_INSTANCE_STYLE_OFFSET)?;
    let style = core::array::from_fn(|index| {
        let at = index * FLOAT_BYTES;
        f32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
    });
    Ok(Some(EffectLook {
        style,
        extra: memory.read_f32(instance + EFFECT_INSTANCE_EXTRA_OFFSET)?,
        flag: memory.read_u8(instance + EFFECT_INSTANCE_CHANGED_FLAG_OFFSET)?,
    }))
}

pub fn write_effect_look<M: GameMemory>(
    memory: &mut M,
    handle: u32,
    look: &EffectLook,
) -> Result<bool, GameError> {
    let Some(instance) = effect_instance_address(memory, handle)? else {
        return Ok(false);
    };
    let bytes: Vec<u8> = look
        .style
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    memory.write(instance + EFFECT_INSTANCE_STYLE_OFFSET, &bytes)?;
    memory.write(
        instance + EFFECT_INSTANCE_EXTRA_OFFSET,
        &look.extra.to_le_bytes(),
    )?;
    memory.write_u8(instance + EFFECT_INSTANCE_CHANGED_FLAG_OFFSET, 1)?;
    Ok(true)
}

pub fn look_log_line(name: &str, sender: &EffectLook, receiver: &EffectLook) -> String {
    format!(
        "effects: effect look {name} sender alpha={} flag={} receiver alpha={} flag={}",
        sender.style[STYLE_ALPHA_INDEX],
        sender.flag,
        receiver.style[STYLE_ALPHA_INDEX],
        receiver.flag
    )
}

#[derive(Debug, Default)]
pub struct OriginNames {
    seen: HashSet<(String, u32)>,
}

impl OriginNames {
    pub fn is_first(&mut self, name: &str, caller_rva: u32) -> bool {
        self.seen.insert((name.to_owned(), caller_rva))
    }
}
