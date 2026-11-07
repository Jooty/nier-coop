use std::collections::HashMap;

use rc_proto::EnemyId;

use crate::bullet_flights::{BULLET_STATE_RUNNING, SightedBullet, read_bullet};
use crate::enemies::ActorHandle;
use crate::enemy_bullets::MAX_SIGHTED_ENEMY_BULLETS;
use crate::game::GameError;
use crate::memory::GameMemory;
use crate::offsets::{
    HANDLE_POOL_POINTERS_RVA, HANDLE_POOL_SLOT_COUNT, NORMAL_BULLET_OWNER_HANDLE_OFFSET,
    NORMAL_BULLET_STATE_OFFSET, NORMAL_BULLET_VTABLE_RVA,
};
use crate::puppet::actor_is_live;

const POINTER_BYTES: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnemyBullet {
    pub enemy: EnemyId,
    pub bullet: SightedBullet,
}

/// Every running, drawn bullet in the handle pool whose owner is one of `enemies` (actor address
/// to shared enemy id), at most `MAX_SIGHTED_ENEMY_BULLETS`.
pub fn bullets_of_enemies<M: GameMemory>(
    memory: &M,
    enemies: &HashMap<u64, EnemyId>,
) -> Result<Vec<EnemyBullet>, GameError> {
    let mut sighted = Vec::new();
    if enemies.is_empty() {
        return Ok(sighted);
    }
    let mut table = vec![0_u8; HANDLE_POOL_SLOT_COUNT as usize * POINTER_BYTES];
    memory.read(
        memory.module_base() + u64::from(HANDLE_POOL_POINTERS_RVA),
        &mut table,
    )?;
    let bullet_vtable = memory.module_base() + u64::from(NORMAL_BULLET_VTABLE_RVA);
    let (pointers, _) = table.as_chunks::<POINTER_BYTES>();
    for (slot, bytes) in (0_u16..).zip(pointers) {
        let actor = u64::from_le_bytes(*bytes);
        if actor == 0 || memory.read_u64(actor) != Ok(bullet_vtable) {
            continue;
        }
        let Some(enemy) = ActorHandle::read(memory, actor + NORMAL_BULLET_OWNER_HANDLE_OFFSET)
            .and_then(|handle| handle.resolve(memory))
            .and_then(|owner| enemies.get(&owner).copied())
        else {
            continue;
        };
        if !actor_is_live(memory, actor).unwrap_or(false)
            || memory.read_i32(actor + NORMAL_BULLET_STATE_OFFSET) != Ok(BULLET_STATE_RUNNING)
        {
            continue;
        }
        let Some(bullet) = read_bullet(memory, actor, slot) else {
            continue;
        };
        sighted.push(EnemyBullet { enemy, bullet });
        if sighted.len() >= MAX_SIGHTED_ENEMY_BULLETS {
            break;
        }
    }
    Ok(sighted)
}
