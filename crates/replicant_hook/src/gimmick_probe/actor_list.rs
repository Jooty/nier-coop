use replicant_game::GameMemory;
use replicant_game::offsets::{ACTOR_TRANSFORM_BLOCK_BYTES, ACTOR_TRANSFORM_BLOCK_OFFSET};
use replicant_game::puppet::PuppetPose;

use crate::memory::InProcessMemory;

pub(super) fn actor_position(memory: &InProcessMemory, actor: u64) -> Option<[f32; 3]> {
    let block = memory
        .read_array::<ACTOR_TRANSFORM_BLOCK_BYTES>(actor + ACTOR_TRANSFORM_BLOCK_OFFSET)
        .ok()?;
    let pose = PuppetPose::from_transform_block(&block);
    Some([pose.position.x, pose.position.y, pose.position.z])
}
