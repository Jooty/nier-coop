use core::ffi::c_void;
use core::ptr;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

use automata_game::address::{
    FIND_STORY_ACTION_BY_ID_FUNCTION, GameAddress, RESET_ACTION_FUNCTION,
    SET_ACTION_STATE_BY_HASH_FUNCTION, SET_ACTION_STATE_FUNCTION,
};
use automata_game::partner::code_starts_with;
use automata_game::story_steps::{
    ACTION_DONE_FLAG, ACTION_DONE_STATE_HASH, ACTION_FLAGS_OFFSET, ACTION_HOLDS_ITS_SEQUENCE_FLAG,
    ACTION_ID_OFFSET, ACTION_STARTED_FLAG, ACTION_STATE_HASH_OFFSET,
    FIND_STORY_ACTION_BY_ID_FUNCTION_START, ObservedDone, RESET_ACTION_FUNCTION_START,
    SET_ACTION_STATE_BY_HASH_FUNCTION_START, SET_ACTION_STATE_FUNCTION_START,
    STATE_ARGUMENT_HASH_OFFSET, STATE_ARGUMENT_VALUE_OFFSET, StepClassLookup, StepClassRow,
    StepVerdict, story_steps_start_differs_line,
};
use nt_hook::diagnostics::log_line;

use crate::game_calls::GameCalls;
use crate::game_hooks::story_gate::call_is_ours;
use crate::story_steps_driver::{
    consult_the_steps_machine_on_a_done, consult_the_steps_machine_on_leaving_done,
    tell_the_steps_machine_of_a_reset,
};

const STATE_UNCHANGED: u8 = 0;

static ORIGINAL_SET_STATE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_RESET: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static WATCHING: AtomicBool = AtomicBool::new(false);
static GUARDS_DONES: AtomicBool = AtomicBool::new(false);
static HOOKS: OnceLock<StepHooks> = OnceLock::new();

type SetState = unsafe extern "system" fn(usize, usize) -> u8;
type Reset = unsafe extern "system" fn(usize);

#[derive(Debug)]
struct StepHooks {
    module_base: usize,
    lookup: StepClassLookup,
}

#[derive(Debug, Clone, Copy)]
struct BlockingAction {
    id: u32,
    flags: u32,
    class: Option<&'static StepClassRow>,
    vftable_rva: u32,
}

/// Hooks `Action.setState` and `Action.reset` after checking the start of every function this
/// stage hooks or calls; installs nothing when one differs. `guards_dones` is mode `on`.
pub fn install_story_step_hooks(
    calls: &GameCalls,
    lookup: StepClassLookup,
    guards_dones: bool,
) -> bool {
    let checked = [
        (
            "setState",
            SET_ACTION_STATE_FUNCTION,
            &SET_ACTION_STATE_FUNCTION_START[..],
        ),
        (
            "setState by hash",
            SET_ACTION_STATE_BY_HASH_FUNCTION,
            &SET_ACTION_STATE_BY_HASH_FUNCTION_START[..],
        ),
        (
            "action reset",
            RESET_ACTION_FUNCTION,
            &RESET_ACTION_FUNCTION_START[..],
        ),
        (
            "find action by id",
            FIND_STORY_ACTION_BY_ID_FUNCTION,
            &FIND_STORY_ACTION_BY_ID_FUNCTION_START[..],
        ),
    ];
    for (label, function, noted_start) in checked {
        let starts_as_noted = calls
            .read_bytes(function, noted_start.len())
            .is_ok_and(|found| code_starts_with(&found, noted_start));
        if !starts_as_noted {
            log_line(&story_steps_start_differs_line(
                label,
                calls.address_value(function),
            ));
            return false;
        }
    }
    let module_base = calls
        .address_value(GameAddress::new(0, 0))
        .try_into()
        .unwrap_or_default();
    if HOOKS
        .set(StepHooks {
            module_base,
            lookup,
        })
        .is_err()
    {
        return false;
    }
    GUARDS_DONES.store(guards_dones, Ordering::Release);
    hook(
        "action setState (story steps)",
        calls,
        SET_ACTION_STATE_FUNCTION,
        set_state_detour as *const (),
        &ORIGINAL_SET_STATE,
    ) && hook(
        "action reset (story steps)",
        calls,
        RESET_ACTION_FUNCTION,
        reset_detour as *const (),
        &ORIGINAL_RESET,
    )
}

/// While false the detours only call the originals (solo, free roam, no partner).
pub fn set_story_steps_watching(watching: bool) {
    WATCHING.store(watching, Ordering::Release);
}

fn hook(
    label: &str,
    calls: &GameCalls,
    function: GameAddress,
    replacement: *const (),
    original: &AtomicPtr<c_void>,
) -> bool {
    #[expect(
        unsafe_code,
        reason = "hooks a game function; each replacement has the signature of its original"
    )]
    // SAFETY: the address is a function of the verified build whose start was just checked, and
    // the replacement given for it takes and returns what the original does.
    unsafe {
        nt_hook::detour::hook_address(
            label,
            calls.address_value(function) as usize,
            replacement as *mut c_void,
            original,
        )
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original setState through the trampoline MinHook built"
)]
extern "system" fn set_state_detour(action: usize, state: usize) -> u8 {
    if WATCHING.load(Ordering::Acquire)
        && action != 0
        && state != 0
        && verdict_on(action, state) == StepVerdict::Refuse
    {
        return STATE_UNCHANGED;
    }
    let original = ORIGINAL_SET_STATE.load(Ordering::Acquire);
    if original.is_null() {
        return STATE_UNCHANGED;
    }
    // SAFETY: the trampoline continues into the original function with the game's arguments.
    unsafe { core::mem::transmute::<*mut c_void, SetState>(original)(action, state) }
}

#[expect(
    unsafe_code,
    reason = "calls the original reset through the trampoline MinHook built"
)]
extern "system" fn reset_detour(action: usize) {
    let watched = if WATCHING.load(Ordering::Acquire) && action != 0 {
        blocking_action(action).filter(|blocking| {
            blocking.flags & (ACTION_STARTED_FLAG | ACTION_DONE_FLAG) != 0
                && blocking.class.is_some_and(|row| row.sort.is_mirrored())
        })
    } else {
        None
    };
    let original = ORIGINAL_RESET.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original function with the game's action.
    unsafe { core::mem::transmute::<*mut c_void, Reset>(original)(action) };
    // The reset may refuse (an action of a hap that is off): only a cleared start counts.
    if let Some(blocking) = watched
        && read_u32(action + ACTION_FLAGS_OFFSET) & ACTION_STARTED_FLAG == 0
    {
        tell_the_steps_machine_of_a_reset(blocking.id);
    }
}

fn verdict_on(action: usize, state: usize) -> StepVerdict {
    let new_hash = read_u32(state + STATE_ARGUMENT_HASH_OFFSET);
    let current_hash = read_u32(action + ACTION_STATE_HASH_OFFSET);
    let becomes_done = new_hash == ACTION_DONE_STATE_HASH && current_hash != ACTION_DONE_STATE_HASH;
    let leaves_done = current_hash == ACTION_DONE_STATE_HASH
        && new_hash != ACTION_DONE_STATE_HASH
        && GUARDS_DONES.load(Ordering::Acquire);
    if !becomes_done && !leaves_done {
        return StepVerdict::Pass;
    }
    let Some(blocking) = blocking_action(action) else {
        return StepVerdict::Pass;
    };
    if leaves_done {
        if !blocking.class.is_some_and(|row| row.sort.is_mirrored()) {
            return StepVerdict::Pass;
        }
        return consult_the_steps_machine_on_leaving_done(blocking.id);
    }
    consult_the_steps_machine_on_a_done(&ObservedDone {
        id: blocking.id,
        class: blocking.class,
        vftable_rva: blocking.vftable_rva,
        value: read_i32(state + STATE_ARGUMENT_VALUE_OFFSET),
        ours: call_is_ours(),
    })
}

fn blocking_action(action: usize) -> Option<BlockingAction> {
    let flags = read_u32(action + ACTION_FLAGS_OFFSET);
    if flags & ACTION_HOLDS_ITS_SEQUENCE_FLAG == 0 {
        return None;
    }
    let hooks = HOOKS.get()?;
    let vftable_rva = u32::try_from(read_usize(action).checked_sub(hooks.module_base)?).ok()?;
    Some(BlockingAction {
        id: read_u32(action + ACTION_ID_OFFSET),
        flags,
        class: hooks.lookup.row_of_vftable_rva(vftable_rva),
        vftable_rva,
    })
}

#[expect(
    unsafe_code,
    reason = "reads a field of a live action or state the game passed to the hooked function"
)]
fn read_u32(address: usize) -> u32 {
    // SAFETY: the address is inside the action or state struct the game passed to setState or
    // reset; both originals read the same fields (the noted starts read state +0 and +0x10 and
    // action +0x20 and +0x30), and +0x00, +0x18 and +0x1C lie inside every action.
    unsafe { ptr::read_unaligned(address as *const u32) }
}

#[expect(
    unsafe_code,
    reason = "reads a field of a live state the game passed to setState"
)]
fn read_i32(address: usize) -> i32 {
    // SAFETY: as for `read_u32`: the state value at +0x10 is read by the original too.
    unsafe { ptr::read_unaligned(address as *const i32) }
}

#[expect(
    unsafe_code,
    reason = "reads the vftable pointer of a live action the game passed to the hooked function"
)]
fn read_usize(address: usize) -> usize {
    // SAFETY: every action is a C++ object whose first field is its vftable pointer.
    unsafe { ptr::read_unaligned(address as *const usize) }
}
