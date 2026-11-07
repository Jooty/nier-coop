use crate::enemies::ActorHandle;
use crate::game::GameError;
use crate::memory::GameMemory;
use crate::offsets::{
    ACTOR_TRANSFORM_BLOCK_OFFSET, ACTOR_TRANSFORM_ROW_STRIDE, BOOK_ACTOR_POINTER_RVA,
    NORMAL_BULLET_OWNER_HANDLE_OFFSET, NORMAL_BULLET_VTABLE_RVA,
};

const TRANSLATION_LANE_BYTES: u64 = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectSource {
    Player,
    Book,
    BulletOfPlayer,
    BulletOfOther,
    Other,
}

pub fn classify_effect_source<M: GameMemory>(
    memory: &M,
    actor: u64,
    player: u64,
) -> Result<EffectSource, GameError> {
    if actor == 0 {
        return Ok(EffectSource::Other);
    }
    if actor == player {
        return Ok(EffectSource::Player);
    }
    let base = memory.module_base();
    let book = memory.read_u64(base + u64::from(BOOK_ACTOR_POINTER_RVA))?;
    if book != 0 && actor == book {
        return Ok(EffectSource::Book);
    }
    if !is_normal_bullet(memory, actor)? {
        return Ok(EffectSource::Other);
    }
    Ok(match bullet_owner(memory, actor) {
        Some(owner) if owner == player => EffectSource::BulletOfPlayer,
        _ => EffectSource::BulletOfOther,
    })
}

pub fn is_normal_bullet<M: GameMemory>(memory: &M, actor: u64) -> Result<bool, GameError> {
    let vtable = memory.read_u64(actor)?;
    Ok(vtable == memory.module_base() + u64::from(NORMAL_BULLET_VTABLE_RVA))
}

pub fn bullet_owner<M: GameMemory>(memory: &M, bullet: u64) -> Option<u64> {
    if !is_normal_bullet(memory, bullet).ok()? {
        return None;
    }
    ActorHandle::read(memory, bullet + NORMAL_BULLET_OWNER_HANDLE_OFFSET)?.resolve(memory)
}

pub fn read_actor_position<M: GameMemory>(memory: &M, actor: u64) -> Result<[f32; 3], GameError> {
    let first_row = actor + ACTOR_TRANSFORM_BLOCK_OFFSET + TRANSLATION_LANE_BYTES;
    let mut position = [0.0_f32; 3];
    for (row, axis) in (0_u64..).zip(position.iter_mut()) {
        *axis = memory.read_f32(first_row + row * ACTOR_TRANSFORM_ROW_STRIDE as u64)?;
    }
    Ok(position)
}
