use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicPtr, Ordering};

use rc_game::GameMemory;
use rc_game::offsets::{
    ACTOR_TRANSFORM_BLOCK_OFFSET, ADD_WORD_DROP_FUNCTION_RVA, DROP_ITEM_MANAGER_RVA,
    WORD_DROP_MATRIX_BYTES,
};
use rc_proto::{EnemyWordDrop, MAX_WORD_ID, Message};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::enemies::EnemySyncRole;
use crate::enemies::combat_view::{applying_remote, with_view};
use crate::enemies::enemy_id_text;
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;
use crate::net;

static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
static ORIGINAL_WORD_DROP: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

type AddWordDrop = unsafe extern "system" fn(*mut c_void, *const u8, u32) -> u64;

pub(super) fn install(build: VerifiedBuild) {
    if HOOKED_BUILD.set(build).is_err() {
        return;
    }
    #[expect(unsafe_code, reason = "hooks DropItemMan::AddWordDrop by its address")]
    // SAFETY: the RVA starts AddWordDrop (drop manager, 64-byte matrix, word id) in the verified
    // build, the call an enemy's death makes after its word roll; the replacement has the same
    // signature.
    unsafe {
        hook_address(
            "DropItemMan::AddWordDrop",
            (build.module_base() + u64::from(ADD_WORD_DROP_FUNCTION_RVA)) as usize,
            add_word_drop_detour as *const () as *mut c_void,
            &ORIGINAL_WORD_DROP,
        );
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original AddWordDrop through the trampoline MinHook built"
)]
pub(super) fn word_drop_at_actor(actor: u64, word_id: u16) -> bool {
    let original = ORIGINAL_WORD_DROP.load(Ordering::Acquire);
    let Some(build) = HOOKED_BUILD.get() else {
        return false;
    };
    if original.is_null() || actor == 0 {
        return false;
    }
    let memory = InProcessMemory::new(build);
    let Ok(matrix) =
        memory.read_array::<WORD_DROP_MATRIX_BYTES>(actor + ACTOR_TRANSFORM_BLOCK_OFFSET)
    else {
        return false;
    };
    let manager = (build.module_base() + u64::from(DROP_ITEM_MANAGER_RVA)) as usize;
    // SAFETY: the trampoline continues into the original AddWordDrop with its exact signature;
    // the matrix is a local copy the game reads before returning, the manager is the game's
    // global drop manager and this runs on the game thread.
    unsafe {
        core::mem::transmute::<*mut c_void, AddWordDrop>(original)(
            manager as *mut c_void,
            matrix.as_ptr(),
            u32::from(word_id),
        ) != 0
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original AddWordDrop through the trampoline MinHook built"
)]
extern "system" fn add_word_drop_detour(
    manager: *mut c_void,
    matrix: *const u8,
    word_id: u32,
) -> u64 {
    let keep = applying_remote()
        || panic::catch_unwind(|| route_word_drop(matrix as u64, word_id)).unwrap_or(true);
    if !keep {
        return 0;
    }
    let original = ORIGINAL_WORD_DROP.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    // SAFETY: the trampoline continues into the original AddWordDrop, which has this exact
    // signature.
    unsafe { core::mem::transmute::<*mut c_void, AddWordDrop>(original)(manager, matrix, word_id) }
}

fn route_word_drop(matrix: u64, word_id: u32) -> bool {
    // An enemy hands its own transform block as the matrix, so the actor sits just below it.
    let actor = matrix.wrapping_sub(ACTOR_TRANSFORM_BLOCK_OFFSET);
    with_view(|view| {
        let Some(id) = view.enemies.get(&actor).copied() else {
            return true;
        };
        match view.role {
            None => true,
            Some(EnemySyncRole::Guest) => {
                log_line(&format!(
                    "combat: held back word {word_id} from your copy of {}; the host's drop is mirrored",
                    enemy_id_text(id)
                ));
                false
            }
            Some(EnemySyncRole::Host) => {
                let Ok(word_id) = u16::try_from(word_id) else {
                    return true;
                };
                if word_id > MAX_WORD_ID {
                    log_line(&format!(
                        "combat: {} dropped word {word_id}, not mirrored (out of range)",
                        enemy_id_text(id)
                    ));
                    return true;
                }
                if let Some(net) = net::handle() {
                    net.send_relayed(Message::EnemyWordDrop(EnemyWordDrop {
                        phase: view.phase,
                        id,
                        word_id,
                    }));
                    log_line(&format!(
                        "combat: {} dropped word {word_id} (sent)",
                        enemy_id_text(id)
                    ));
                }
                true
            }
        }
    })
}
