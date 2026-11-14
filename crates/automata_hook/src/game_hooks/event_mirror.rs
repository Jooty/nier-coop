use core::cell::Cell;
use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicPtr, Ordering};

use automata_game::address::{
    EVENT_END_HOOK_FUNCTION, EVENT_LOAD_AND_PLAY_FUNCTION, EVENT_SKIP_FUNCTION,
    EVENT_START_HOOK_FUNCTION, GameAddress,
};
use automata_game::event_follow::{
    EVENT_END_HOOK_FUNCTION_START, EVENT_LOAD_AND_PLAY_FUNCTION_START, EVENT_SKIP_FUNCTION_START,
    EVENT_START_HOOK_FUNCTION_START, PAUSE_STATE_TAKING_AN_EVENT_SKIP, PlayVerdict, SkipVerdict,
};
use automata_game::partner::code_starts_with;
use nt_hook::diagnostics::log_line;

use crate::event_follow_driver::{
    consult_the_event_machine_on_a_play, consult_the_event_machine_on_a_skip,
    tell_the_event_machine_of_a_start, tell_the_event_machine_of_an_end,
};
use crate::game_calls::GameCalls;

static ORIGINAL_START_HOOK: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_END_HOOK: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_SKIP: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_LOAD_AND_PLAY: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static MIRROR_CALLS: OnceLock<GameCalls> = OnceLock::new();

thread_local! {
    static EVENT_CALL_IS_OURS: Cell<bool> = const { Cell::new(false) };
}

type OneArgument = unsafe extern "system" fn(usize) -> usize;
type TwoArguments = unsafe extern "system" fn(usize, usize) -> usize;
type NoArgument = unsafe extern "system" fn() -> usize;

/// Hooks the event start and end points, the skip function and the play request; false, with
/// nothing hooked, when any of the four function starts differs.
pub fn install_event_mirror(calls: GameCalls) -> bool {
    let functions: [(&str, GameAddress, &[u8], *mut c_void, &AtomicPtr<c_void>); 4] = [
        (
            "event start point (event mirror)",
            EVENT_START_HOOK_FUNCTION,
            &EVENT_START_HOOK_FUNCTION_START,
            start_hook_detour as *const () as *mut c_void,
            &ORIGINAL_START_HOOK,
        ),
        (
            "event end point (event mirror)",
            EVENT_END_HOOK_FUNCTION,
            &EVENT_END_HOOK_FUNCTION_START,
            end_hook_detour as *const () as *mut c_void,
            &ORIGINAL_END_HOOK,
        ),
        (
            "event skip function (event mirror)",
            EVENT_SKIP_FUNCTION,
            &EVENT_SKIP_FUNCTION_START,
            skip_detour as *const () as *mut c_void,
            &ORIGINAL_SKIP,
        ),
        (
            "event play request (event mirror)",
            EVENT_LOAD_AND_PLAY_FUNCTION,
            &EVENT_LOAD_AND_PLAY_FUNCTION_START,
            load_and_play_detour as *const () as *mut c_void,
            &ORIGINAL_LOAD_AND_PLAY,
        ),
    ];
    let differing: Vec<&str> = functions
        .iter()
        .filter(|(_, function, start, _, _)| {
            !calls
                .read_bytes(*function, start.len())
                .is_ok_and(|found| code_starts_with(&found, start))
        })
        .map(|(label, _, _, _, _)| *label)
        .collect();
    if !differing.is_empty() {
        log_line(&format!(
            "event: the start bytes of {} are not the expected ones, the event mirror is off",
            differing.join(", ")
        ));
        return false;
    }
    if MIRROR_CALLS.set(calls).is_err() {
        return false;
    }
    let mut hooked = 0;
    for (label, function, _, replacement, original) in functions {
        #[expect(
            unsafe_code,
            reason = "hooks an event function by address; each replacement has the original's signature"
        )]
        // SAFETY: the address is an event function of the verified build whose start was just
        // checked; each replacement forwards its arguments and result to the original.
        let done = unsafe {
            nt_hook::detour::hook_address(
                label,
                calls.address_value(function) as usize,
                replacement,
                original,
            )
        };
        hooked += usize::from(done);
    }
    // A missing hook leaves the others forwarding unchanged; the machine then sees less.
    hooked == functions.len()
}

pub fn run_event_call_as_ours<T>(call: impl FnOnce() -> T) -> T {
    let before = EVENT_CALL_IS_OURS.with(|flag| flag.replace(true));
    let result = call();
    EVENT_CALL_IS_OURS.with(|flag| flag.set(before));
    result
}

#[expect(
    unsafe_code,
    reason = "calls the original function through the trampoline MinHook built"
)]
fn call_original_with_one(slot: &AtomicPtr<c_void>, argument: usize) -> usize {
    let original = slot.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    // SAFETY: the trampoline continues into the original function with the argument received.
    unsafe { core::mem::transmute::<*mut c_void, OneArgument>(original)(argument) }
}

extern "system" fn start_hook_detour(data_id: usize) -> usize {
    let id = MIRROR_CALLS
        .get()
        .and_then(|calls| calls.read_event_id_at(data_id as u64));
    let result = call_original_with_one(&ORIGINAL_START_HOOK, data_id);
    if let Some(id) = id {
        let _ = panic::catch_unwind(|| tell_the_event_machine_of_a_start(id));
    }
    result
}

#[expect(
    unsafe_code,
    reason = "calls the original end point through the trampoline MinHook built"
)]
extern "system" fn end_hook_detour(data_id: usize, skipped: usize) -> usize {
    let id = MIRROR_CALLS
        .get()
        .and_then(|calls| calls.read_event_id_at(data_id as u64));
    let original = ORIGINAL_END_HOOK.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    // SAFETY: the trampoline continues into the original function with the arguments received.
    let result =
        unsafe { core::mem::transmute::<*mut c_void, TwoArguments>(original)(data_id, skipped) };
    if let Some(id) = id {
        let skipped = skipped as u32 != 0;
        let _ = panic::catch_unwind(|| tell_the_event_machine_of_an_end(id, skipped));
    }
    result
}

#[expect(
    unsafe_code,
    reason = "calls the original skip function through the trampoline MinHook built"
)]
extern "system" fn skip_detour() -> usize {
    let original = ORIGINAL_SKIP.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    let ours = EVENT_CALL_IS_OURS.with(Cell::get);
    // Only the pause manager's skip state is a player's skip; the script's own stop always passes.
    let by_the_player = !ours
        && MIRROR_CALLS
            .get()
            .and_then(GameCalls::pause_state)
            .is_some_and(|state| state == PAUSE_STATE_TAKING_AN_EVENT_SKIP);
    let swallowed = by_the_player
        && panic::catch_unwind(consult_the_event_machine_on_a_skip)
            .is_ok_and(|verdict| verdict == SkipVerdict::Swallow);
    if swallowed {
        return 0;
    }
    // SAFETY: the trampoline continues into the original function, which takes no argument.
    unsafe { core::mem::transmute::<*mut c_void, NoArgument>(original)() }
}

extern "system" fn load_and_play_detour(data_id: usize) -> usize {
    let result = call_original_with_one(&ORIGINAL_LOAD_AND_PLAY, data_id);
    if result != 0 && !EVENT_CALL_IS_OURS.with(Cell::get) {
        let _ = panic::catch_unwind(|| hold_the_play_if_the_rule_says(data_id as u64));
    }
    result
}

fn hold_the_play_if_the_rule_says(data_id: u64) {
    let Some(calls) = MIRROR_CALLS.get() else {
        return;
    };
    if !calls.is_a_phase_event_actions_data_id(data_id) {
        return;
    }
    let Some(id) = calls.read_event_id_at(data_id) else {
        return;
    };
    if consult_the_event_machine_on_a_play(id) == PlayVerdict::Hold && !calls.hold_slot_play(id) {
        log_line(&format!(
            "event: {} was to be held but no slot holds it, it plays",
            id.describe()
        ));
    }
}
