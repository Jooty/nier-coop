use core::ffi::c_void;

use rc_game::enemies::{partner_actors, scan_enemies};
use rc_game::offsets::{ACTOR_MANAGER_RVA, GET_ACTOR_IN_GROUP_FUNCTION_RVA};
use rc_game::{GameMemory, Replicant};

use crate::build_gate::VerifiedBuild;
use crate::memory::InProcessMemory;

type GetActorInGroup = unsafe extern "system" fn(*mut c_void, i32, i32) -> *mut c_void;

const LIVE_ACTOR_HP_PLACEHOLDER: (i32, i32) = (1, 1);

pub(super) fn live_enemy_ids(build: &VerifiedBuild) -> Vec<(u16, i32)> {
    let memory = InProcessMemory::new(build);
    let game = Replicant::new(InProcessMemory::new(build));
    let module_base = memory.module_base();
    let sightings = scan_enemies(
        &memory,
        game.playable_actor().ok(),
        &partner_actors(&memory),
        |group, index| actor_in_group(module_base, group, index),
        |_| Some(LIVE_ACTOR_HP_PLACEHOLDER),
    )
    .unwrap_or_default();
    sightings
        .iter()
        .map(|sighting| (sighting.group, sighting.class_id))
        .collect()
}

fn actor_in_group(module_base: u64, group: u16, index: i32) -> u64 {
    let function = (module_base + u64::from(GET_ACTOR_IN_GROUP_FUNCTION_RVA)) as usize;
    let manager = (module_base + u64::from(ACTOR_MANAGER_RVA)) as usize;
    #[expect(
        unsafe_code,
        reason = "calls CActorManager::GetActorInGroup at its verified address"
    )]
    // SAFETY: the build gate passed, so the RVA starts GetActorInGroup (manager, group, index ->
    // actor or null), which bounds-checks both numbers itself. This runs on the game thread inside
    // the script manager tick, where scripts call it too.
    unsafe {
        core::mem::transmute::<usize, GetActorInGroup>(function)(
            manager as *mut c_void,
            i32::from(group),
            index,
        ) as usize as u64
    }
}
