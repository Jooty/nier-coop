use rc_proto::{ENEMY_GROUP_COUNT, NPC_UNGROUPED};

use crate::game::GameError;
use crate::memory::GameMemory;
use crate::npcs::NpcSighting;
use crate::offsets::{
    ACTOR_CLASS_ID_OFFSET, ACTOR_DRAWN_FLAG_OFFSET, ACTOR_LIFE_STATE_OFFSET, ACTOR_MANAGER_RVA,
    ACTOR_MANAGER_SECONDARY_LIST_HEAD_OFFSET, ACTOR_SCRIPT_GROUP_OFFSET,
    ACTOR_SECONDARY_LIST_NEXT_OFFSET, ACTOR_SPAWN_SERIAL_OFFSET, ACTOR_TRANSFORM_BLOCK_BYTES,
    ACTOR_TRANSFORM_BLOCK_OFFSET, BODY_MOTION_LAYER, HUMAN_NPC_VTABLE_RVA,
    MAX_ACTORS_WALKED_IN_SECONDARY_LIST,
};
use crate::puppet::{PuppetPose, actor_is_live, read_layer_motion};

/// Lists every live villager (`CActorHumanNpc`) on the actor manager's secondary list, which
/// holds every actor that can be ticked, in list order. Only memory reads, no game calls.
pub fn scan_npcs<M: GameMemory>(memory: &M) -> Result<Vec<NpcSighting>, GameError> {
    let base = memory.module_base();
    let npc_vtable = base + u64::from(HUMAN_NPC_VTABLE_RVA);
    let mut actor = memory
        .read_u64(base + u64::from(ACTOR_MANAGER_RVA) + ACTOR_MANAGER_SECONDARY_LIST_HEAD_OFFSET)?;
    let mut sightings = Vec::new();
    for _ in 0..MAX_ACTORS_WALKED_IN_SECONDARY_LIST {
        if actor == 0 {
            break;
        }
        if memory.read_u64(actor).ok() == Some(npc_vtable)
            && let Some(sighting) = sight(memory, actor)
        {
            sightings.push(sighting);
        }
        actor = memory.read_u64(actor + ACTOR_SECONDARY_LIST_NEXT_OFFSET)?;
    }
    Ok(sightings)
}

fn sight<M: GameMemory>(memory: &M, actor: u64) -> Option<NpcSighting> {
    if !actor_is_live(memory, actor).ok()? {
        return None;
    }
    // The manager's own lookups skip actors whose life state is above zero (5 is destroyed).
    if memory.read_i32(actor + ACTOR_LIFE_STATE_OFFSET).ok()? > 0 {
        return None;
    }
    let group = u16::try_from(memory.read_i32(actor + ACTOR_SCRIPT_GROUP_OFFSET).ok()?)
        .ok()
        .filter(|group| *group < ENEMY_GROUP_COUNT)
        .unwrap_or(NPC_UNGROUPED);
    let class = u16::try_from(memory.read_i32(actor + ACTOR_CLASS_ID_OFFSET).ok()?).ok()?;
    let block = memory
        .read_array::<ACTOR_TRANSFORM_BLOCK_BYTES>(actor + ACTOR_TRANSFORM_BLOCK_OFFSET)
        .ok()?;
    let pose = PuppetPose::from_transform_block(&block);
    Some(NpcSighting {
        actor,
        spawn_serial: memory.read_i32(actor + ACTOR_SPAWN_SERIAL_OFFSET).ok()?,
        group,
        class,
        position: [pose.position.x, pose.position.y, pose.position.z],
        yaw_radians: pose.yaw,
        motion: read_layer_motion(memory, actor, BODY_MOTION_LAYER)
            .ok()
            .flatten(),
        visible: memory.read_u8(actor + ACTOR_DRAWN_FLAG_OFFSET).ok()? != 0,
    })
}
