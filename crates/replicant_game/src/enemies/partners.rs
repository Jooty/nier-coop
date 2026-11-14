use crate::memory::GameMemory;
use crate::offsets::{
    ACTOR_TRANSFORM_BLOCK_BYTES, ACTOR_TRANSFORM_BLOCK_OFFSET, PARTNER_KAINE_ACTOR_RVA,
    PARTNER_THIRD_MEMBER_ACTOR_RVA, PARTNER_VARIANT_POINTER_RVA,
};
use crate::puppet::{PuppetPose, actor_is_live};

pub fn partner_actors<M: GameMemory>(memory: &M) -> Vec<u64> {
    let base = memory.module_base();
    let variant = memory
        .read_u64(base + u64::from(PARTNER_VARIANT_POINTER_RVA))
        .unwrap_or(0);
    [
        base + u64::from(PARTNER_KAINE_ACTOR_RVA),
        base + u64::from(PARTNER_THIRD_MEMBER_ACTOR_RVA),
        variant,
    ]
    .into_iter()
    .filter(|actor| *actor != 0)
    .collect()
}

/// Unique actor -2's place and facing, when the game holds her live.
pub fn unique_companion_pose<M: GameMemory>(memory: &M) -> Option<PuppetPose> {
    let actor = memory.module_base() + u64::from(PARTNER_KAINE_ACTOR_RVA);
    if !actor_is_live(memory, actor).unwrap_or(false) {
        return None;
    }
    let block = memory
        .read_array::<ACTOR_TRANSFORM_BLOCK_BYTES>(actor + ACTOR_TRANSFORM_BLOCK_OFFSET)
        .ok()?;
    Some(PuppetPose::from_transform_block(&block))
}
