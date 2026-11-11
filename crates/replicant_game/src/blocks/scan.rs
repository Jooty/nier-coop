use replicant_proto::ENEMY_GROUP_COUNT;

use crate::blocks::{BLOCK_VTABLE_RVA, BlockSighting, read_block_position};
use crate::enemies::read_group_counts;
use crate::memory::GameMemory;
use crate::offsets::MAX_ACTORS_SCANNED_PER_GROUP;

pub fn block_is_live<M: GameMemory>(memory: &M, actor: u64) -> bool {
    actor != 0
        && memory
            .read_u64(actor)
            .is_ok_and(|vtable| vtable == memory.module_base() + u64::from(BLOCK_VTABLE_RVA))
}

pub fn read_block_sightings<M: GameMemory>(
    memory: &M,
    mut actor_in_group: impl FnMut(u16, i32) -> u64,
) -> Vec<BlockSighting> {
    let Ok(counts) = read_group_counts(memory) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for (group, count) in (0..ENEMY_GROUP_COUNT).zip(counts) {
        for index in 0..count.clamp(0, MAX_ACTORS_SCANNED_PER_GROUP) {
            let actor = actor_in_group(group, index);
            if !block_is_live(memory, actor) {
                continue;
            }
            if let Some(position) = read_block_position(memory, actor) {
                found.push(BlockSighting { actor, position });
            }
        }
    }
    found
}
