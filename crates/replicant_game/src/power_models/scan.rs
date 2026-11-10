use crate::enemies::ActorHandle;
use crate::game::GameError;
use crate::memory::GameMemory;
use crate::offsets::{
    ACTOR_TRANSFORM_BLOCK_BYTES, ACTOR_TRANSFORM_BLOCK_OFFSET, BLACK_HAND_OWNER_HANDLE_OFFSET,
    BLACK_HAND_VTABLE_RVA, HANDLE_POOL_POINTERS_RVA, HANDLE_POOL_SLOT_COUNT,
};
use crate::power_models::MAX_SIGHTED_HANDS;
use crate::puppet::{PuppetPose, actor_is_live};

const POINTER_BYTES: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SightedHand {
    pub actor: u64,
    pub slot: u16,
    pub pose: PuppetPose,
}

pub fn hands_of_player<M: GameMemory>(
    memory: &M,
    player: u64,
) -> Result<Vec<SightedHand>, GameError> {
    let mut table = vec![0_u8; HANDLE_POOL_SLOT_COUNT as usize * POINTER_BYTES];
    memory.read(
        memory.module_base() + u64::from(HANDLE_POOL_POINTERS_RVA),
        &mut table,
    )?;
    let hand_vtable = memory.module_base() + u64::from(BLACK_HAND_VTABLE_RVA);
    let mut sighted = Vec::new();
    let (pointers, _) = table.as_chunks::<POINTER_BYTES>();
    for (slot, bytes) in (0_u16..).zip(pointers) {
        let actor = u64::from_le_bytes(*bytes);
        if actor == 0 || memory.read_u64(actor) != Ok(hand_vtable) {
            continue;
        }
        let owner = ActorHandle::read(memory, actor + BLACK_HAND_OWNER_HANDLE_OFFSET)
            .and_then(|handle| handle.resolve(memory));
        if owner != Some(player) || !actor_is_live(memory, actor).unwrap_or(false) {
            continue;
        }
        let Ok(block) =
            memory.read_array::<ACTOR_TRANSFORM_BLOCK_BYTES>(actor + ACTOR_TRANSFORM_BLOCK_OFFSET)
        else {
            continue;
        };
        sighted.push(SightedHand {
            actor,
            slot,
            pose: PuppetPose::from_transform_block(&block),
        });
        if sighted.len() >= MAX_SIGHTED_HANDS {
            break;
        }
    }
    Ok(sighted)
}
