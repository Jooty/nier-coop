use std::collections::HashMap;

use replicant_proto::ENEMY_GROUP_COUNT;

use crate::blocks::read_block_position;
use crate::bombs::pool_actors;
use crate::enemies::read_group_counts;
use crate::memory::GameMemory;
use crate::offsets::{ACTOR_CLASS_ID_OFFSET, MAX_ACTORS_SCANNED_PER_GROUP};
use crate::puppet::actor_of_handle;
use crate::world_objects::{
    HandleOwner, ObjectSighting, UNGROUPED_OBJECT_GROUP, kind_of_vtable_rva,
};

pub fn object_is_live<M: GameMemory>(memory: &M, actor: u64, kind_vtable_rva: u32) -> bool {
    actor != 0 && vtable_rva_of(memory, actor) == Some(kind_vtable_rva)
}

/// Every listed object in the game's handle pool, with the script group whose list holds it
/// (`UNGROUPED_OBJECT_GROUP` when no list does).
pub fn read_object_sightings<M: GameMemory>(
    memory: &M,
    actor_in_group: impl FnMut(u16, i32) -> u64,
) -> Vec<ObjectSighting> {
    let Ok(actors) = pool_actors(memory) else {
        return Vec::new();
    };
    let listed: Vec<(u64, u32)> = actors
        .into_iter()
        .filter_map(|(_, actor)| {
            let rva = vtable_rva_of(memory, actor)?;
            kind_of_vtable_rva(rva).map(|_| (actor, rva))
        })
        .collect();
    if listed.is_empty() {
        return Vec::new();
    }
    let groups = groups_of_listed(memory, &listed, actor_in_group);
    listed
        .into_iter()
        .filter_map(|(actor, rva)| {
            Some(ObjectSighting {
                actor,
                kind_vtable_rva: rva,
                group: groups
                    .get(&actor)
                    .copied()
                    .unwrap_or(UNGROUPED_OBJECT_GROUP),
                position: read_block_position(memory, actor)?,
            })
        })
        .collect()
}

/// The actor a held self handle names, of any class, as the handle pool resolves it.
pub fn handle_owner<M: GameMemory>(memory: &M, handle: u64) -> Option<HandleOwner> {
    let index = (handle & u64::from(u32::MAX)) as u32;
    let generation = (handle >> 32) as u32;
    let actor = actor_of_handle(memory, index, generation).ok().flatten()?;
    Some(HandleOwner {
        actor,
        vtable_rva: vtable_rva_of(memory, actor).unwrap_or(0),
        class_id: memory.read_i32(actor + ACTOR_CLASS_ID_OFFSET).unwrap_or(-1),
        position: read_block_position(memory, actor),
    })
}

fn vtable_rva_of<M: GameMemory>(memory: &M, actor: u64) -> Option<u32> {
    let vtable = memory.read_u64(actor).ok()?;
    u32::try_from(vtable.checked_sub(memory.module_base())?).ok()
}

fn groups_of_listed<M: GameMemory>(
    memory: &M,
    listed: &[(u64, u32)],
    mut actor_in_group: impl FnMut(u16, i32) -> u64,
) -> HashMap<u64, u16> {
    let mut groups = HashMap::new();
    let Ok(counts) = read_group_counts(memory) else {
        return groups;
    };
    for (group, count) in (0..ENEMY_GROUP_COUNT).zip(counts) {
        for index in 0..count.clamp(0, MAX_ACTORS_SCANNED_PER_GROUP) {
            let actor = actor_in_group(group, index);
            if listed.iter().any(|(known, _)| *known == actor) {
                groups.entry(actor).or_insert(group);
            }
        }
    }
    groups
}
