use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicPtr, Ordering};

use replicant_game::GameMemory;
use replicant_game::offsets::{
    ACTOR_TRANSFORM_BLOCK_BYTES, ACTOR_TRANSFORM_BLOCK_OFFSET, ADD_DROP_AT_ACTOR_FUNCTION_RVA,
    DROP_ITEM_MANAGER_RVA,
};
use replicant_game::puppet::PuppetPose;
use replicant_proto::{EnemyDrop, MAX_ITEM_COUNT, Message};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::enemies::EnemySyncRole;
use crate::enemies::combat_view::{applying_remote, with_view};
use crate::enemies::counters::{HookCount, count};
use crate::enemies::enemy_id_text;
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;
use crate::net;

static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
static ORIGINAL_DROP: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

type AddDropAtActor = unsafe extern "system" fn(*mut c_void, *mut c_void, i32, i32) -> u64;

pub(super) fn install(build: VerifiedBuild) {
    if HOOKED_BUILD.set(build).is_err() {
        return;
    }
    #[expect(
        unsafe_code,
        reason = "hooks DropItemMan::AddDropAtActor by its address"
    )]
    // SAFETY: the RVA starts AddDropAtActor (drop manager, actor, item id, count) in the verified
    // build, the call an enemy's death drop makes; the replacement has the same signature.
    unsafe {
        hook_address(
            "DropItemMan::AddDropAtActor",
            (build.module_base() + u64::from(ADD_DROP_AT_ACTOR_FUNCTION_RVA)) as usize,
            add_drop_detour as *const () as *mut c_void,
            &ORIGINAL_DROP,
        );
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original AddDropAtActor through the trampoline MinHook built"
)]
pub(super) fn drop_at_actor(actor: u64, item_id: u16, count: u8) -> bool {
    let original = ORIGINAL_DROP.load(Ordering::Acquire);
    let Some(build) = HOOKED_BUILD.get() else {
        return false;
    };
    if original.is_null() || actor == 0 {
        return false;
    }
    let manager = (build.module_base() + u64::from(DROP_ITEM_MANAGER_RVA)) as usize;
    // SAFETY: the trampoline continues into the original AddDropAtActor with its exact
    // signature; the manager is the game's global drop manager, the actor is live this tick and
    // this runs on the game thread.
    unsafe {
        core::mem::transmute::<*mut c_void, AddDropAtActor>(original)(
            manager as *mut c_void,
            actor as usize as *mut c_void,
            i32::from(item_id),
            i32::from(count),
        ) != 0
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original AddDropAtActor through the trampoline MinHook built"
)]
extern "system" fn add_drop_detour(
    manager: *mut c_void,
    actor: *mut c_void,
    item_id: i32,
    item_count: i32,
) -> u64 {
    let keep = applying_remote()
        || panic::catch_unwind(|| route_drop(actor as u64, item_id, item_count)).unwrap_or(true);
    if !keep {
        return 0;
    }
    let original = ORIGINAL_DROP.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    // SAFETY: the trampoline continues into the original AddDropAtActor, which has this exact
    // signature.
    unsafe {
        core::mem::transmute::<*mut c_void, AddDropAtActor>(original)(
            manager, actor, item_id, item_count,
        )
    }
}

fn route_drop(actor: u64, item_id: i32, item_count: i32) -> bool {
    let Some(build) = HOOKED_BUILD.get() else {
        return true;
    };
    with_view(|view| {
        let Some(id) = view.enemies.get(&actor).copied() else {
            return true;
        };
        match view.role {
            None => true,
            Some(EnemySyncRole::Guest) => {
                count(HookCount::CopyDropBlocked);
                log_line(&format!(
                    "combat: held back item {item_id} x{item_count} from your copy of {}; the host's drop is mirrored",
                    enemy_id_text(id)
                ));
                false
            }
            Some(EnemySyncRole::Host) => {
                let memory = InProcessMemory::new(build);
                let position = memory
                    .read_array::<ACTOR_TRANSFORM_BLOCK_BYTES>(actor + ACTOR_TRANSFORM_BLOCK_OFFSET)
                    .ok()
                    .map(|block| PuppetPose::from_transform_block(&block).position)
                    .map_or([0.0; 3], |at| [at.x, at.y, at.z]);
                let (Ok(item_id), Ok(drop_count)) =
                    (u16::try_from(item_id), u8::try_from(item_count))
                else {
                    log_line(&format!(
                        "combat: {} dropped item {item_id} x{item_count}, not mirrored (out of range)",
                        enemy_id_text(id)
                    ));
                    return true;
                };
                if let Some(net) = net::handle()
                    && drop_count > 0
                    && drop_count <= MAX_ITEM_COUNT
                {
                    net.send_relayed(Message::EnemyDrop(EnemyDrop {
                        phase: view.phase,
                        id,
                        item_id,
                        count: drop_count,
                        position,
                    }));
                    count(HookCount::DropSent);
                    log_line(&format!(
                        "combat: {} dropped item {item_id} x{drop_count} at ({:.1}, {:.1}, {:.1}) (sent)",
                        enemy_id_text(id),
                        position[0],
                        position[1],
                        position[2]
                    ));
                }
                true
            }
        }
    })
}
