use replicant_proto::{
    BULLET_EFFECT_SLOTS, BULLET_STYLE_FLOATS, BULLET_TRANSFORM_FLOATS, BulletLook,
};

use crate::bullet_flights::{SightedBullet, partner_effect_id};
use crate::effects::effect_instance_address;
use crate::memory::GameMemory;
use crate::offsets::{
    EFFECT_INSTANCE_MATRIX_OFFSET, EFFECT_INSTANCE_STYLE_OFFSET, NORMAL_BULLET_HIT_ID_OFFSET,
    SCRIPT_BULLET_EFFECT_OBJECT_EFFECT_ID_OFFSET, SCRIPT_BULLET_EFFECT_OBJECT_HANDLE_OFFSET,
    SCRIPT_BULLET_EFFECT_OBJECT_POINTER_OFFSET, SCRIPT_BULLET_LOOP_EFFECT_VTABLE_RVA,
    SCRIPT_BULLET_SPRITE_EFFECT_VTABLE_RVA, SCRIPT_BULLET_TO_SENIOR_EFFECT_VTABLE_RVA,
};

const FLOAT_BYTES: usize = 4;
const STYLE_BYTES: usize = BULLET_STYLE_FLOATS * FLOAT_BYTES;
const TRANSFORM_BYTES: usize = BULLET_TRANSFORM_FLOATS * FLOAT_BYTES;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotCopyable {
    Sprite,
    NoEffect,
}

/// A script bullet is drawn by one effect object: a looping effect made by id, or a sprite.
/// Only the looping effect can be shown again from its id, at the place the effect itself is.
pub fn read_script_bullet<M: GameMemory>(
    memory: &M,
    actor: u64,
    slot: u16,
) -> Result<SightedBullet, NotCopyable> {
    let object = memory
        .read_u64(actor + SCRIPT_BULLET_EFFECT_OBJECT_POINTER_OFFSET)
        .ok()
        .filter(|object| *object != 0)
        .ok_or(NotCopyable::NoEffect)?;
    let class = memory.read_u64(object).map_err(|_| NotCopyable::NoEffect)?;
    let base = memory.module_base();
    if class == base + u64::from(SCRIPT_BULLET_SPRITE_EFFECT_VTABLE_RVA) {
        return Err(NotCopyable::Sprite);
    }
    if class != base + u64::from(SCRIPT_BULLET_LOOP_EFFECT_VTABLE_RVA)
        && class != base + u64::from(SCRIPT_BULLET_TO_SENIOR_EFFECT_VTABLE_RVA)
    {
        return Err(NotCopyable::NoEffect);
    }
    read_looping_effect(memory, actor, object, slot).ok_or(NotCopyable::NoEffect)
}

fn read_looping_effect<M: GameMemory>(
    memory: &M,
    actor: u64,
    object: u64,
    slot: u16,
) -> Option<SightedBullet> {
    let effect_id = memory
        .read_i32(object + SCRIPT_BULLET_EFFECT_OBJECT_EFFECT_ID_OFFSET)
        .ok()?;
    partner_effect_id(effect_id)?;
    let handle = memory
        .read_i32(object + SCRIPT_BULLET_EFFECT_OBJECT_HANDLE_OFFSET)
        .ok()?
        .cast_unsigned();
    let instance = effect_instance_address(memory, handle).ok().flatten()?;
    let hit_id = memory.read_i32(actor + NORMAL_BULLET_HIT_ID_OFFSET).ok()?;
    let transform = memory
        .read_array::<TRANSFORM_BYTES>(instance + EFFECT_INSTANCE_MATRIX_OFFSET)
        .ok()?;
    let style = memory
        .read_array::<STYLE_BYTES>(instance + EFFECT_INSTANCE_STYLE_OFFSET)
        .ok()?;
    let mut effect_ids = [0_i32; BULLET_EFFECT_SLOTS];
    let mut styles = [[0.0_f32; BULLET_STYLE_FLOATS]; BULLET_EFFECT_SLOTS];
    effect_ids[0] = effect_id;
    styles[0] = floats_of(&style);
    Some(SightedBullet {
        actor,
        slot,
        hit_id,
        transform: floats_of(&transform),
        look: BulletLook { effect_ids, styles },
    })
}

fn floats_of<const FLOATS: usize>(bytes: &[u8]) -> [f32; FLOATS] {
    core::array::from_fn(|index| {
        let at = index * FLOAT_BYTES;
        f32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
    })
}
