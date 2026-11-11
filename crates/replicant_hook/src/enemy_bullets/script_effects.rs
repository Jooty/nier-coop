use core::cell::Cell;
use core::ffi::c_void;
use core::ptr;
use std::collections::HashSet;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{LazyLock, Mutex, OnceLock, PoisonError};

use replicant_game::effects::{EffectNameTally, RateWindow};
use replicant_game::enemies::ActorHandle;
use replicant_game::enemy_bullets::{
    MAX_BULLET_SCRIPT_EFFECTS_SENT_PER_SECOND, SCRIPT_BULLET_COMMAND_RECEIVER_OFFSET,
    SCRIPT_BULLET_COMMANDS_THAT_MAKE_ONE_SHOT_EFFECTS, SCRIPT_BULLET_EXECUTE_COMMAND_RVA,
    bullet_script_effects_line,
};
use replicant_game::offsets::NORMAL_BULLET_OWNER_HANDLE_OFFSET;

use crate::build_gate::VerifiedBuild;
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;
use crate::net::net_clock_ms;

const RATE_WINDOW_MS: u64 = 1000;

static ORIGINAL_EXECUTE_COMMAND: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
static SHARED_ENEMY_ACTORS: LazyLock<Mutex<HashSet<u64>>> = LazyLock::new(Mutex::default);
static SEND_WINDOW: LazyLock<Mutex<RateWindow>> = LazyLock::new(|| {
    Mutex::new(RateWindow::new(
        RATE_WINDOW_MS,
        MAX_BULLET_SCRIPT_EFFECTS_SENT_PER_SECOND,
    ))
});
static TALLY: LazyLock<Mutex<EffectNameTally>> = LazyLock::new(Mutex::default);

thread_local! {
    static COMMAND_IN_FLIGHT: Cell<Option<(u64, i32)>> = const { Cell::new(None) };
}

type ExecuteCommand = unsafe extern "system" fn(u64, i32, u64, u32, u64, u64) -> u64;

pub fn install(build: VerifiedBuild) {
    if HOOKED_BUILD.set(build).is_err() {
        return;
    }
    let address = (build.module_base() + u64::from(SCRIPT_BULLET_EXECUTE_COMMAND_RVA)) as usize;
    #[expect(
        unsafe_code,
        reason = "hooks the script bullet's command interpreter by its address"
    )]
    // SAFETY: the RVA starts `CScriptBullet::ExecuteScriptCommand` in the verified build (docs
    // symbols.tsv): six integer or pointer arguments, the first the bullet + 0x1900, and an
    // integer result. The replacement has the same signature and forwards every argument.
    unsafe {
        hook_address(
            "CScriptBullet::ExecuteScriptCommand",
            address,
            execute_command_detour as *const () as *mut c_void,
            &ORIGINAL_EXECUTE_COMMAND,
        );
    }
}

/// The host's shared enemies this scan; an empty set while not hosting a partner in play.
pub(crate) fn set_shared_enemy_actors(actors: impl Iterator<Item = u64>) {
    let mut shared = SHARED_ENEMY_ACTORS
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    shared.clear();
    shared.extend(actors);
}

/// `true` inside a bullet script command that makes a one-shot effect, for a bullet a shared
/// enemy of this host fired.
pub(crate) fn in_enemy_bullet_effect_command() -> bool {
    let Some((bullet, command)) = COMMAND_IN_FLIGHT.with(Cell::get) else {
        return false;
    };
    if !SCRIPT_BULLET_COMMANDS_THAT_MAKE_ONE_SHOT_EFFECTS.contains(&command) {
        return false;
    }
    let Some(build) = HOOKED_BUILD.get() else {
        return false;
    };
    let memory = InProcessMemory::new(build);
    let Some(owner) = ActorHandle::read(&memory, bullet + NORMAL_BULLET_OWNER_HANDLE_OFFSET)
        .and_then(|handle| handle.resolve(&memory))
    else {
        return false;
    };
    SHARED_ENEMY_ACTORS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .contains(&owner)
}

/// Counts one effect a shared enemy's bullet script made and says whether it may be sent now.
pub(crate) fn admit_bullet_script_effect(name: &str) -> bool {
    let admitted = SEND_WINDOW
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .admit(net_clock_ms());
    TALLY
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .note(name, admitted);
    admitted
}

pub(super) fn take_summary_line() -> Option<String> {
    bullet_script_effects_line(&mut TALLY.lock().unwrap_or_else(PoisonError::into_inner))
}

extern "system" fn execute_command_detour(
    receiver: u64,
    command: i32,
    arguments: u64,
    fourth: u32,
    fifth: u64,
    sixth: u64,
) -> u64 {
    let trampoline = ORIGINAL_EXECUTE_COMMAND.load(Ordering::Acquire);
    if trampoline.is_null() {
        return 0;
    }
    let bullet = receiver.wrapping_sub(SCRIPT_BULLET_COMMAND_RECEIVER_OFFSET);
    let outer = COMMAND_IN_FLIGHT.with(|cell| cell.replace(Some((bullet, command))));
    let result = call_original(
        trampoline, receiver, command, arguments, fourth, fifth, sixth,
    );
    COMMAND_IN_FLIGHT.with(|cell| cell.set(outer));
    result
}

#[expect(
    unsafe_code,
    reason = "calls the original interpreter through the trampoline"
)]
fn call_original(
    trampoline: *mut c_void,
    receiver: u64,
    command: i32,
    arguments: u64,
    fourth: u32,
    fifth: u64,
    sixth: u64,
) -> u64 {
    // SAFETY: the trampoline continues into the original interpreter, which has this signature.
    unsafe {
        core::mem::transmute::<*mut c_void, ExecuteCommand>(trampoline)(
            receiver, command, arguments, fourth, fifth, sixth,
        )
    }
}
