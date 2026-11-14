use core::ffi::c_void;
use core::ptr;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicPtr, AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};

use automata_game::address::{PLAYER_BODY_VTABLE, START_ANIMATION_FUNCTION};
use automata_game::defeat::local_game_may_start_on_the_held_body;
use automata_game::downed::start_is_refused_in_the_downed_pose;
use automata_game::minute_line_budget::MinuteLineBudget;
use automata_game::partner::{
    BODY_HELD_BUTTONS_OFFSET, BODY_INPUT_IGNORE_TIMER_OFFSET, BODY_SPEED_OFFSET, BODY_STATE_OFFSET,
    OwnDashCut, RecordedStart, START_ANIMATION_FUNCTION_START, STATE_START_VTABLE_OFFSET,
    code_starts_with, is_a_dash_state, own_dash_cut_line, start_cuts_a_dash,
};
use nt_hook::diagnostics::log_line;
use windows_sys::Win32::System::Diagnostics::Debug::RtlCaptureStackBackTrace;

use crate::budgeted_log::log_within;
use crate::game_calls::GameCalls;
use crate::game_hooks::held_body_update_mark::a_held_body_update_runs;
use crate::game_hooks::partner_walk_guards::guarded_slot_body;

const OWN_DASH_CUTS_LOGGED_PER_MINUTE: u32 = 20;
const STACK_FRAMES_SEARCHED_FOR_GAME_CODE: usize = 12;
const GAME_CODE_RVAS_LOGGED_AT_MOST: usize = 5;
const GAME_IMAGE_RVA_LIMIT: u64 = 0x200_0000;

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
static HERO_IS_KEPT_DOWN: AtomicBool = AtomicBool::new(false);
static HELD_BODY_IS_KEPT_DOWN: AtomicBool = AtomicBool::new(false);
static FIRST_START_REFUSED_ON_THE_DOWNED_HERO_LOGGED: AtomicBool = AtomicBool::new(false);
static FIRST_START_REFUSED_ON_THE_DOWNED_PARTNER_BODY_LOGGED: AtomicBool = AtomicBool::new(false);
static HELD_BUTTONS_AT_THE_DASH_START: AtomicU32 = AtomicU32::new(0);
static A_DASH_START_IS_SEEN: AtomicBool = AtomicBool::new(false);
static OWN_DASH_CUTS_LOGGED: AtomicU32 = AtomicU32::new(0);
static OWN_DASH_CUT_LINES: Mutex<MinuteLineBudget> =
    Mutex::new(MinuteLineBudget::new(OWN_DASH_CUTS_LOGGED_PER_MINUTE));

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
                nt_hook::detour::hook_address(
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

/// Which bodies stay in the downed pose state from now on; game thread only.
pub fn keep_bodies_in_the_downed_pose(hero: bool, held_body: bool) {
    HERO_IS_KEPT_DOWN.store(hero, Ordering::Relaxed);
    HELD_BODY_IS_KEPT_DOWN.store(held_body, Ordering::Relaxed);
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
    let is_the_hero = RECORDER_CALLS
        .get()
        .is_some_and(|calls| calls.current_hero() == body);
    let is_the_held_body = guarded_slot_body().is_some_and(|(_, held)| held == body);
    if start_is_refused_on_a_downed_body(body, state, is_the_hero, is_the_held_body) {
        return 0;
    }
    if is_the_hero {
        watch_the_own_dash(body, state);
        record(state, variant, b, c);
    }
    if !local_game_may_start_on_the_held_body(state) && is_the_held_body {
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

fn start_is_refused_on_a_downed_body(
    body: usize,
    state: i32,
    is_the_hero: bool,
    is_the_held_body: bool,
) -> bool {
    let kept_down = (is_the_hero && HERO_IS_KEPT_DOWN.load(Ordering::Relaxed))
        || (is_the_held_body && HELD_BODY_IS_KEPT_DOWN.load(Ordering::Relaxed));
    let refused = kept_down
        && RECORDER_CALLS.get().is_some_and(|calls| {
            calls
                .read_i32_at(calls.body_field(body, BODY_STATE_OFFSET))
                .is_ok_and(|current| start_is_refused_in_the_downed_pose(kept_down, current))
        });
    if refused {
        let (first_logged, whose) = if is_the_hero {
            (
                &FIRST_START_REFUSED_ON_THE_DOWNED_HERO_LOGGED,
                "the downed hero",
            )
        } else {
            (
                &FIRST_START_REFUSED_ON_THE_DOWNED_PARTNER_BODY_LOGGED,
                "the downed other player's body",
            )
        };
        if !first_logged.swap(true, Ordering::Relaxed) {
            log_line(&format!(
                "downed: the game wanted to start state {state:#X} on {whose}; refused, the body stays in the pose state until it is revived or the game's defeat takes over (logged once)"
            ));
        }
    }
    refused
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

fn watch_the_own_dash(hero: usize, started: i32) {
    let Some(calls) = RECORDER_CALLS.get() else {
        return;
    };
    let read_int = |offset| calls.read_i32_at(calls.body_field(hero, offset));
    let Ok(current) = read_int(BODY_STATE_OFFSET) else {
        return;
    };
    let held_now = calls
        .read_u32_at(calls.body_field(hero, BODY_HELD_BUTTONS_OFFSET))
        .unwrap_or(0);
    if is_a_dash_state(started) && !is_a_dash_state(current) {
        HELD_BUTTONS_AT_THE_DASH_START.store(held_now, Ordering::Relaxed);
        A_DASH_START_IS_SEEN.store(true, Ordering::Relaxed);
        return;
    }
    if !start_cuts_a_dash(current, started) {
        return;
    }
    let number = OWN_DASH_CUTS_LOGGED.fetch_add(1, Ordering::Relaxed) + 1;
    log_within(&OWN_DASH_CUT_LINES, "own dash", || {
        let read_float = |offset| {
            calls
                .read_f32_at(calls.body_field(hero, offset))
                .unwrap_or(f32::NAN)
        };
        let cut = OwnDashCut {
            from_state: current,
            to_state: started,
            held_buttons_now: held_now,
            held_buttons_at_the_dash_start: A_DASH_START_IS_SEEN
                .load(Ordering::Relaxed)
                .then(|| HELD_BUTTONS_AT_THE_DASH_START.load(Ordering::Relaxed)),
            speed: read_float(BODY_SPEED_OFFSET),
            input_ignore_seconds: read_float(BODY_INPUT_IGNORE_TIMER_OFFSET),
            inside_a_held_body_update: a_held_body_update_runs(),
            caller_rvas: game_code_on_the_stack(calls),
        };
        own_dash_cut_line(&cut, number, OWN_DASH_CUTS_LOGGED_PER_MINUTE)
    });
}

#[expect(
    unsafe_code,
    reason = "walks the stack to find the game code that started a state on the hero"
)]
fn game_code_on_the_stack(calls: &GameCalls) -> Vec<u32> {
    let image_base = calls
        .address_value(START_ANIMATION_FUNCTION)
        .wrapping_sub(u64::from(START_ANIMATION_FUNCTION.rva(calls.variant())));
    let mut frames = [ptr::null_mut::<c_void>(); STACK_FRAMES_SEARCHED_FOR_GAME_CODE];
    // SAFETY: the buffer holds the frames asked for; the hash out-pointer may be null.
    let captured = unsafe {
        RtlCaptureStackBackTrace(
            0,
            STACK_FRAMES_SEARCHED_FOR_GAME_CODE as u32,
            frames.as_mut_ptr(),
            ptr::null_mut(),
        )
    };
    frames
        .iter()
        .take(usize::from(captured))
        .map(|frame| (*frame as usize as u64).wrapping_sub(image_base))
        .filter(|rva| *rva < GAME_IMAGE_RVA_LIMIT)
        .filter_map(|rva| u32::try_from(rva).ok())
        .take(GAME_CODE_RVAS_LOGGED_AT_MOST)
        .collect()
}
