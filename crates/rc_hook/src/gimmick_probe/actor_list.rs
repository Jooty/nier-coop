use core::ffi::c_void;

use rc_game::GameMemory;
use rc_game::offsets::{
    ACTOR_MANAGER_RVA, ACTOR_TRANSFORM_BLOCK_BYTES, ACTOR_TRANSFORM_BLOCK_OFFSET,
    GET_ACTOR_IN_GROUP_FUNCTION_RVA,
};
use rc_game::puppet::PuppetPose;

use crate::build_gate::VerifiedBuild;
use crate::memory::InProcessMemory;

type GetActorInGroup = unsafe extern "system" fn(*mut c_void, i32, i32) -> *mut c_void;

pub(crate) fn actor_in_group(build: &VerifiedBuild, group: u16, index: i32) -> u64 {
    let module_base = build.module_base();
    let function = (module_base + u64::from(GET_ACTOR_IN_GROUP_FUNCTION_RVA)) as usize;
    let manager = (module_base + u64::from(ACTOR_MANAGER_RVA)) as usize;
    #[expect(
        unsafe_code,
        reason = "calls CActorManager::GetActorInGroup at its verified address"
    )]
    // SAFETY: the build gate passed, so the RVA starts GetActorInGroup (manager, group, index ->
    // actor or null), which bounds-checks both numbers itself. This runs on the game thread
    // inside the script manager tick, where scripts call it too.
    unsafe {
        core::mem::transmute::<usize, GetActorInGroup>(function)(
            manager as *mut c_void,
            i32::from(group),
            index,
        ) as usize as u64
    }
}

pub(super) fn actor_position(memory: &InProcessMemory, actor: u64) -> Option<[f32; 3]> {
    let block = memory
        .read_array::<ACTOR_TRANSFORM_BLOCK_BYTES>(actor + ACTOR_TRANSFORM_BLOCK_OFFSET)
        .ok()?;
    let pose = PuppetPose::from_transform_block(&block);
    Some([pose.position.x, pose.position.y, pose.position.z])
}
