use rc_proto::{
    BULLET_EFFECT_SLOTS, BULLET_STYLE_FLOATS, BULLET_TRANSFORM_FLOATS, BulletLook,
    MAX_BULLET_EFFECT_ID,
};

use crate::bullet_flights::{BULLET_STATE_RUNNING, MAX_SIGHTED_BULLETS};
use crate::effects::effect_instance_address;
use crate::enemies::ActorHandle;
use crate::game::GameError;
use crate::memory::GameMemory;
use crate::offsets::{
    ACTOR_TRANSFORM_BLOCK_BYTES, ACTOR_TRANSFORM_BLOCK_OFFSET, EFFECT_INSTANCE_STYLE_OFFSET,
    HANDLE_POOL_POINTERS_RVA, HANDLE_POOL_SLOT_COUNT, NORMAL_BULLET_EFFECT_HANDLES_OFFSET,
    NORMAL_BULLET_HIT_ID_OFFSET, NORMAL_BULLET_OWNER_HANDLE_OFFSET,
    NORMAL_BULLET_RECORD_EFFECT_IDS_OFFSET, NORMAL_BULLET_RECORD_POINTER_OFFSET,
    NORMAL_BULLET_STATE_OFFSET, NORMAL_BULLET_VTABLE_RVA,
};
use crate::puppet::actor_is_live;

const POINTER_BYTES: usize = 8;
const FLOAT_BYTES: usize = 4;
const STYLE_BYTES: usize = BULLET_STYLE_FLOATS * FLOAT_BYTES;
const STYLE_ALPHA_INDEX: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SightedBullet {
    pub actor: u64,
    pub slot: u16,
    pub hit_id: i32,
    pub transform: [f32; BULLET_TRANSFORM_FLOATS],
    pub look: BulletLook,
}

pub fn bullets_of_player<M: GameMemory>(
    memory: &M,
    player: u64,
) -> Result<Vec<SightedBullet>, GameError> {
    let mut table = vec![0_u8; HANDLE_POOL_SLOT_COUNT as usize * POINTER_BYTES];
    memory.read(
        memory.module_base() + u64::from(HANDLE_POOL_POINTERS_RVA),
        &mut table,
    )?;
    let bullet_vtable = memory.module_base() + u64::from(NORMAL_BULLET_VTABLE_RVA);
    let mut sighted = Vec::new();
    let (pointers, _) = table.as_chunks::<POINTER_BYTES>();
    for (slot, bytes) in (0_u16..).zip(pointers) {
        let actor = u64::from_le_bytes(*bytes);
        if actor == 0 || memory.read_u64(actor) != Ok(bullet_vtable) {
            continue;
        }
        let owner = ActorHandle::read(memory, actor + NORMAL_BULLET_OWNER_HANDLE_OFFSET)
            .and_then(|handle| handle.resolve(memory));
        if owner != Some(player) || !actor_is_live(memory, actor).unwrap_or(false) {
            continue;
        }
        if memory.read_i32(actor + NORMAL_BULLET_STATE_OFFSET) != Ok(BULLET_STATE_RUNNING) {
            continue;
        }
        let Some(bullet) = read_bullet(memory, actor, slot) else {
            continue;
        };
        sighted.push(bullet);
        if sighted.len() >= MAX_SIGHTED_BULLETS {
            break;
        }
    }
    Ok(sighted)
}

pub fn read_bullet<M: GameMemory>(memory: &M, actor: u64, slot: u16) -> Option<SightedBullet> {
    let record = memory
        .read_u64(actor + NORMAL_BULLET_RECORD_POINTER_OFFSET)
        .ok()
        .filter(|record| *record != 0)?;
    let hit_id = memory.read_i32(actor + NORMAL_BULLET_HIT_ID_OFFSET).ok()?;
    let block = memory
        .read_array::<ACTOR_TRANSFORM_BLOCK_BYTES>(actor + ACTOR_TRANSFORM_BLOCK_OFFSET)
        .ok()?;
    let mut effect_ids = [0_i32; BULLET_EFFECT_SLOTS];
    let mut styles = [[0.0_f32; BULLET_STYLE_FLOATS]; BULLET_EFFECT_SLOTS];
    let mut drawn = false;
    for (index, (id, style)) in (0_u64..).zip(effect_ids.iter_mut().zip(styles.iter_mut())) {
        *id = memory
            .read_i32(record + NORMAL_BULLET_RECORD_EFFECT_IDS_OFFSET + index * 4)
            .ok()?;
        let handle = memory
            .read_i32(actor + NORMAL_BULLET_EFFECT_HANDLES_OFFSET + index * 4)
            .ok()?
            .cast_unsigned();
        let Some(instance) = effect_instance_address(memory, handle).ok().flatten() else {
            continue;
        };
        drawn = true;
        if let Ok(bytes) = memory.read_array::<STYLE_BYTES>(instance + EFFECT_INSTANCE_STYLE_OFFSET)
        {
            *style = floats_of(&bytes);
        }
    }
    if !drawn {
        return None;
    }
    Some(SightedBullet {
        actor,
        slot,
        hit_id,
        transform: floats_of(&block),
        look: BulletLook { effect_ids, styles },
    })
}

fn floats_of<const FLOATS: usize>(bytes: &[u8]) -> [f32; FLOATS] {
    core::array::from_fn(|index| {
        let at = index * FLOAT_BYTES;
        f32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
    })
}

pub fn style_is_set(style: &[f32; BULLET_STYLE_FLOATS]) -> bool {
    style[STYLE_ALPHA_INDEX] > 0.0
}

pub fn partner_effect_id(effect_id: i32) -> Option<u32> {
    u32::try_from(effect_id)
        .ok()
        .filter(|id| (1..=MAX_BULLET_EFFECT_ID.cast_unsigned()).contains(id))
}
