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

pub const SPRITE_EFFECT_OBJECT_BYTES_LOGGED: usize = 0x60;
pub const MAX_SPRITE_BULLET_KINDS_LOGGED: usize = 8;
const FLOAT_BYTES: usize = 4;
const STYLE_BYTES: usize = BULLET_STYLE_FLOATS * FLOAT_BYTES;
const TRANSFORM_BYTES: usize = BULLET_TRANSFORM_FLOATS * FLOAT_BYTES;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotCopyable {
    Sprite,
    NoEffect,
}

/// The first bytes of the sprite object that draws one sprite-drawn script bullet, for the log:
/// nothing can show such a bullet on the partner's game yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpriteBulletSample {
    pub hit_id: i32,
    pub object_bytes: [u8; SPRITE_EFFECT_OBJECT_BYTES_LOGGED],
}

/// Which sprite-drawn bullet kinds (by hit id) were already written to the log.
#[derive(Debug, Clone, Default)]
pub struct SpriteBulletKinds {
    logged_hit_ids: Vec<i32>,
}

impl SpriteBulletSample {
    pub fn text(&self) -> String {
        let words: Vec<String> = self
            .object_bytes
            .as_chunks::<FLOAT_BYTES>()
            .0
            .iter()
            .map(|word| format!("{:08x}", u32::from_le_bytes(*word)))
            .collect();
        format!(
            "hit id {}, sprite object as 32-bit words from +0x0: {}",
            self.hit_id,
            words.join(" ")
        )
    }
}

impl SpriteBulletKinds {
    pub fn first_sighting(&mut self, hit_id: i32) -> bool {
        if self.logged_hit_ids.contains(&hit_id)
            || self.logged_hit_ids.len() >= MAX_SPRITE_BULLET_KINDS_LOGGED
        {
            return false;
        }
        self.logged_hit_ids.push(hit_id);
        true
    }
}

pub fn read_sprite_bullet_hit_id<M: GameMemory>(memory: &M, actor: u64) -> Option<i32> {
    memory.read_i32(actor + NORMAL_BULLET_HIT_ID_OFFSET).ok()
}

pub fn read_sprite_bullet_sample<M: GameMemory>(
    memory: &M,
    actor: u64,
) -> Option<SpriteBulletSample> {
    let object = memory
        .read_u64(actor + SCRIPT_BULLET_EFFECT_OBJECT_POINTER_OFFSET)
        .ok()
        .filter(|object| *object != 0)?;
    Some(SpriteBulletSample {
        hit_id: read_sprite_bullet_hit_id(memory, actor)?,
        object_bytes: memory
            .read_array::<SPRITE_EFFECT_OBJECT_BYTES_LOGGED>(object)
            .ok()?,
    })
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
