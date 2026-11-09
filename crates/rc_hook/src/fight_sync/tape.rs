use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::PoisonError;
use std::sync::atomic::{AtomicPtr, Ordering};

use rc_game::GameMemory;
use rc_game::fight_sync::{
    TAPED_NATIVE_RVAS, TAPED_NATIVES, TAPED_NATIVES_HOOKED_BY_SCRIPT_ROLLS, TapeValue,
};
use rc_game::script_rolls::LUA_CALL_RESULTS_OFFSET;
use rc_game::stay_together::FollowRole;
use rc_game::story_relay::{ARG_VALUE_OFFSET, ARGS_HEADER_BYTES};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::fight_sync::state::{BUILD, GUEST, HOST, here};
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;
use crate::net;

type TapedNative = unsafe extern "system" fn(*mut c_void);

macro_rules! taped_native_detour {
    ($detour:ident, $original:ident, $native:literal) => {
        static $original: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

        extern "system" fn $detour(args: *mut c_void) {
            run_and_tape(args, $native, &$original);
        }
    };
}

taped_native_detour!(
    get_actor_life_detour,
    ORIGINAL_GET_ACTOR_LIFE,
    "_GetActorLife"
);
taped_native_detour!(
    get_actor_life_rate_detour,
    ORIGINAL_GET_ACTOR_LIFE_RATE,
    "_GetActorLifeRate"
);
taped_native_detour!(
    get_unique_actor_max_life_detour,
    ORIGINAL_GET_UNIQUE_ACTOR_MAX_LIFE,
    "_GetUniqueActorMaxLife"
);
taped_native_detour!(
    get_actor_count_detour,
    ORIGINAL_GET_ACTOR_COUNT,
    "_GetActorCount"
);
taped_native_detour!(
    get_actor_count_groups_detour,
    ORIGINAL_GET_ACTOR_COUNT_GROUPS,
    "_GetActorCountGroups"
);
taped_native_detour!(
    get_actor_num_in_script_area_detour,
    ORIGINAL_GET_ACTOR_NUM_IN_SCRIPT_AREA,
    "_GetActorNumInScriptArea"
);
taped_native_detour!(
    get_near_player_enemy_num_detour,
    ORIGINAL_GET_NEAR_PLAYER_ENEMY_NUM,
    "_GetNearPlayerEnemyNum"
);
taped_native_detour!(
    get_global_time_count_detour,
    ORIGINAL_GET_GLOBAL_TIME_COUNT,
    "_GetGlobalTimeCount"
);

type TapedNativeDetour = (
    &'static str,
    extern "system" fn(*mut c_void),
    &'static AtomicPtr<c_void>,
);

static TAPED_NATIVE_DETOURS: [TapedNativeDetour; 8] = [
    (
        "_GetActorLife",
        get_actor_life_detour,
        &ORIGINAL_GET_ACTOR_LIFE,
    ),
    (
        "_GetActorLifeRate",
        get_actor_life_rate_detour,
        &ORIGINAL_GET_ACTOR_LIFE_RATE,
    ),
    (
        "_GetUniqueActorMaxLife",
        get_unique_actor_max_life_detour,
        &ORIGINAL_GET_UNIQUE_ACTOR_MAX_LIFE,
    ),
    (
        "_GetActorCount",
        get_actor_count_detour,
        &ORIGINAL_GET_ACTOR_COUNT,
    ),
    (
        "_GetActorCountGroups",
        get_actor_count_groups_detour,
        &ORIGINAL_GET_ACTOR_COUNT_GROUPS,
    ),
    (
        "_GetActorNumInScriptArea",
        get_actor_num_in_script_area_detour,
        &ORIGINAL_GET_ACTOR_NUM_IN_SCRIPT_AREA,
    ),
    (
        "_GetNearPlayerEnemyNum",
        get_near_player_enemy_num_detour,
        &ORIGINAL_GET_NEAR_PLAYER_ENEMY_NUM,
    ),
    (
        "_GetGlobalTimeCount",
        get_global_time_count_detour,
        &ORIGINAL_GET_GLOBAL_TIME_COUNT,
    ),
];

pub(super) fn install(build: VerifiedBuild) {
    let base = build.module_base();
    for (native, detour, original) in &TAPED_NATIVE_DETOURS {
        let Some(rva) = TAPED_NATIVE_RVAS
            .iter()
            .find(|(name, _)| name == native)
            .map(|(_, rva)| *rva)
        else {
            log_line(&format!("fight tape: no address for {native}, not taped"));
            continue;
        };
        #[expect(unsafe_code, reason = "hooks a state-read script native by address")]
        // SAFETY: each RVA starts the native of that name in the verified build (binding table
        // 0x1306710 walk, docs symbols.tsv); the replacement has the natives' one-pointer signature.
        unsafe {
            hook_address(
                &format!("LuaNative:{native}"),
                (base + u64::from(rva)) as usize,
                *detour as *const () as *mut c_void,
                original,
            );
        }
    }
    let uncovered: Vec<&str> = TAPED_NATIVES
        .iter()
        .copied()
        .filter(|native| {
            !TAPED_NATIVES_HOOKED_BY_SCRIPT_ROLLS.contains(native)
                && !TAPED_NATIVE_DETOURS
                    .iter()
                    .any(|(name, _, _)| name == native)
        })
        .collect();
    if !uncovered.is_empty() {
        log_line(&format!("fight tape: not hooked: {}", uncovered.join(", ")));
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original native through the trampoline"
)]
fn run_and_tape(args: *mut c_void, native: &'static str, original: &AtomicPtr<c_void>) {
    let trampoline = original.load(Ordering::Acquire);
    if trampoline.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original native, which has this signature.
    unsafe { core::mem::transmute::<*mut c_void, TapedNative>(trampoline)(args) };
    tape_native_result(args as u64, native);
}

pub(crate) fn tape_native_result(args: u64, native: &str) -> bool {
    // Right after a taped native returned: inside a streamed handler the host sends the value,
    // inside a replayed handler the guest takes the host's. True when the fight tape took the
    // read, so no other sharing path touches it.
    panic::catch_unwind(|| tape(args, native)).unwrap_or(false)
}

fn tape(args: u64, native: &str) -> bool {
    let Some(build) = BUILD.get() else {
        return false;
    };
    let Some(here) = here(build) else {
        return false;
    };
    let mut memory = InProcessMemory::new(build);
    let result_slot = args + LUA_CALL_RESULTS_OFFSET + ARGS_HEADER_BYTES as u64;
    let local = read_result(&memory, args, result_slot);
    match here.role {
        FollowRole::Host => {
            let out = HOST
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .taped_read(native, local);
            let Some(out) = out else {
                return false;
            };
            if let Some(line) = out.log {
                log_line(&line);
            }
            if let Some(net) = net::handle() {
                net.send_relayed(out.message);
            }
            true
        }
        FollowRole::Guest => {
            let read = GUEST
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .read_taped(native, local, here.local_phase);
            let Some(read) = read else {
                return false;
            };
            if let Some(line) = read.log {
                log_line(&line);
            }
            if read.value != local
                && let Some(payload) = read.value.result_payload()
                && memory
                    .write(result_slot + ARG_VALUE_OFFSET as u64, &payload)
                    .is_err()
            {
                log_line(&format!(
                    "fight tape: {native}'s result could not be written, the game keeps its own"
                ));
            }
            true
        }
        FollowRole::Unknown => false,
    }
}

fn read_result(memory: &InProcessMemory, args: u64, result_slot: u64) -> TapeValue {
    let count = memory.read_i32(args + LUA_CALL_RESULTS_OFFSET).unwrap_or(0);
    let tag = memory.read_i32(result_slot).unwrap_or(-1);
    let payload = memory
        .read_array::<8>(result_slot + ARG_VALUE_OFFSET as u64)
        .unwrap_or_default();
    TapeValue::from_result_slot(count, tag, payload)
}
