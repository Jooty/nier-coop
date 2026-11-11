use core::ffi::c_void;
use core::ptr;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicPtr, AtomicU32, Ordering};

use automata_game::address::{PLAYER_BODY_VTABLE, START_ANIMATION_FUNCTION};
use automata_game::defeat::local_game_may_start_on_the_held_body;
use automata_game::partner::{
    RecordedStart, START_ANIMATION_FUNCTION_START, STATE_START_VTABLE_OFFSET, code_starts_with,
};
use nc_hook::diagnostics::log_line;

use crate::game_calls::GameCalls;
use crate::game_hooks::partner_walk_guards::guarded_held_body;

static ORIGINAL_START_ANIMATION: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static RECORDER_CALLS: OnceLock<GameCalls> = OnceLock::new();
static A_START_IS_RECORDED: AtomicBool = AtomicBool::new(false);
static START_COUNT: AtomicU32 = AtomicU32::new(0);
static STARTED_STATE: AtomicI32 = AtomicI32::new(0);
static STARTED_VARIANT: AtomicU32 = AtomicU32::new(0);
static STARTED_B: AtomicU32 = AtomicU32::new(0);
static STARTED_C: AtomicU32 = AtomicU32::new(0);
static FIRST_RECORD_LOGGED: AtomicBool = AtomicBool::new(false);
static FIRST_REFUSED_DEFEAT_START_LOGGED: AtomicBool = AtomicBool::new(false);

type StartAnimation = unsafe extern "system" fn(usize, i32, u32, u32, u32) -> usize;

/// Records the hero's own animation starts for the other player; false, with nothing hooked,
/// when the function is not the noted one.
pub fn install_start_recorder(calls: GameCalls) -> bool {
    let function = calls.address_value(START_ANIMATION_FUNCTION);
    let start_matches = calls
        .read_bytes(
            START_ANIMATION_FUNCTION,
            START_ANIMATION_FUNCTION_START.len(),
        )
        .is_ok_and(|found| code_starts_with(&found, &START_ANIMATION_FUNCTION_START));
    let slot = calls.address_value(PLAYER_BODY_VTABLE) + u64::from(STATE_START_VTABLE_OFFSET);
    let slot_matches = calls.read_u64_at(slot).is_ok_and(|held| held == function);
    let hooked = start_matches
        && slot_matches
        && RECORDER_CALLS.set(calls).is_ok()
        && {
            #[expect(
                unsafe_code,
                reason = "hooks the body's animation start; the replacement has the signature of the original"
            )]
            // SAFETY: the address is the animation start (vtable slot 18) of the verified build,
            // its start and its vtable slot were just checked, and the replacement forwards all
            // five arguments and the result.
            unsafe {
                nc_hook::detour::hook_address(
                    "player body animation start (records the hero's own starts, no defeat state on the partner body)",
                    function as usize,
                    start_animation_detour as *const () as *mut c_void,
                    &ORIGINAL_START_ANIMATION,
                )
            }
        };
    if !hooked {
        log_line(&format!(
            "partner: the animation start is not the noted one or its hook failed (start bytes match: {start_matches}, vtable slot matches: {slot_matches}); the other player gets the state id only and starts states at phase 0"
        ));
    }
    hooked
}

/// The hero's newest recorded start, in the form the wire takes; game thread only.
pub fn recorded_start() -> Option<RecordedStart> {
    if !A_START_IS_RECORDED.load(Ordering::Acquire) {
        return None;
    }
    RecordedStart::of_call(
        START_COUNT.load(Ordering::Relaxed),
        STARTED_STATE.load(Ordering::Relaxed),
        STARTED_VARIANT.load(Ordering::Relaxed),
        STARTED_B.load(Ordering::Relaxed),
        STARTED_C.load(Ordering::Relaxed),
    )
}

/// Call when the hero is another body than before: the record belonged to the old one.
pub fn forget_recorded_start() {
    A_START_IS_RECORDED.store(false, Ordering::Release);
}

#[expect(
    unsafe_code,
    reason = "calls the original animation start through the trampoline MinHook built"
)]
extern "system" fn start_animation_detour(
    body: usize,
    state: i32,
    variant: u32,
    b: u32,
    c: u32,
) -> usize {
    if RECORDER_CALLS
        .get()
        .is_some_and(|calls| calls.current_hero() == body)
    {
        record(state, variant, b, c);
    }
    if !local_game_may_start_on_the_held_body(state)
        && guarded_held_body().is_some_and(|(_, held)| held == body)
    {
        // With the player flag the hold sets, the body's defeat state asks for this game's own
        // defeat flow.
        if !FIRST_REFUSED_DEFEAT_START_LOGGED.swap(true, Ordering::Relaxed) {
            log_line(&format!(
                "partner: this game wanted to start the defeat state {state:#X} on the partner body; refused, its defeat is the other player's game's to decide (logged once)"
            ));
        }
        return 0;
    }
    let original = ORIGINAL_START_ANIMATION.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    // SAFETY: the trampoline continues into the original function with the arguments received.
    unsafe {
        core::mem::transmute::<*mut c_void, StartAnimation>(original)(body, state, variant, b, c)
    }
}

fn record(state: i32, variant: u32, b: u32, c: u32) {
    STARTED_STATE.store(state, Ordering::Relaxed);
    STARTED_VARIANT.store(variant, Ordering::Relaxed);
    STARTED_B.store(b, Ordering::Relaxed);
    STARTED_C.store(c, Ordering::Relaxed);
    let count = START_COUNT.fetch_add(1, Ordering::Relaxed).wrapping_add(1);
    A_START_IS_RECORDED.store(true, Ordering::Release);
    if !FIRST_RECORD_LOGGED.swap(true, Ordering::Relaxed) {
        log_line(&format!(
            "partner: first animation start of the hero recorded for the other player (state {state:#X}, arguments {variant}, {b}, {c}, start number {count})"
        ));
    }
}
