use core::cell::Cell;
use core::ffi::c_void;
use core::ptr;
use core::time::Duration;
use std::collections::BTreeSet;
use std::sync::Mutex;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::time::Instant;

use automata_game::address::{
    ENEMY_GENERATOR_VFTABLE, GENERATOR_LEAVE_TEST_FUNCTION, GameAddress,
    PLAYER_PLACE_FOR_AREA_TESTS_FUNCTION,
};
use automata_game::enemies::{
    GENERATOR_ACTION_ID_OFFSET, GENERATOR_LEAVE_TEST_AREA_OFFSET,
    GENERATOR_LEAVE_TEST_FUNCTION_START, LEAVE_TEST_LINES_LOGGED_PER_MINUTE,
    LEAVE_TEST_RULE_LAPSES_AFTER_MS, LeaveOutcome, LeaveTestRule, LeaveWatch,
    PLAYER_PLACE_FOR_AREA_TESTS_FUNCTION_START,
};
use automata_game::minute_line_budget::MinuteLineBudget;
use automata_game::partner::code_starts_with;
use nc_hook::diagnostics::log_line;

use crate::budgeted_log::log_owned_within;
use crate::game_calls::GameCalls;
use crate::game_hooks::partner_walk_guards::guarded_slot_body;

const PLAYERS_HAVE_NOT_LEFT: u8 = 0;

static ORIGINAL_LEAVE_TEST: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_PLAYER_PLACE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static LEAVE: Mutex<Option<LeaveState>> = Mutex::new(None);

thread_local! {
    static PLACE_FOR_THIS_AREA_TEST: Cell<Option<[f32; 4]>> = const { Cell::new(None) };
}

type LeaveTest = unsafe extern "system" fn(usize) -> u8;
type PlayerPlace = unsafe extern "system" fn(*mut [f32; 4]) -> *mut [f32; 4];

struct LeaveState {
    calls: GameCalls,
    partner_place_is_hooked: bool,
    rule: LeaveTestRule,
    rule_lapses_at: Option<Instant>,
    generators_left_to_the_game: BTreeSet<u32>,
    watch: LeaveWatch,
    lines: MinuteLineBudget,
}

#[derive(Debug, Clone, Copy)]
struct AskedLeave {
    rule: LeaveTestRule,
    generator: u32,
}

/// Hooks the generators' leave test and, when `partner_place_wanted` (the host), the player
/// place the game's area tests read, so the partner body can be tested too. Gives what was hooked.
pub fn install_generator_leave_test(calls: GameCalls, partner_place_wanted: bool) -> (bool, bool) {
    let leave_starts_as_noted = start_is_the_noted_one(
        &calls,
        GENERATOR_LEAVE_TEST_FUNCTION,
        &GENERATOR_LEAVE_TEST_FUNCTION_START,
    );
    if !leave_starts_as_noted {
        return (false, false);
    }
    let partner_place_is_hooked = partner_place_wanted
        && start_is_the_noted_one(
            &calls,
            PLAYER_PLACE_FOR_AREA_TESTS_FUNCTION,
            &PLAYER_PLACE_FOR_AREA_TESTS_FUNCTION_START,
        )
        && hook(
            "player place of the area tests (the host tests the partner body's place too)",
            &calls,
            PLAYER_PLACE_FOR_AREA_TESTS_FUNCTION,
            player_place_detour as *const (),
            &ORIGINAL_PLAYER_PLACE,
        );
    if let Ok(mut leave) = LEAVE.lock() {
        *leave = Some(LeaveState {
            calls,
            partner_place_is_hooked,
            rule: LeaveTestRule::TheGamesOwn,
            rule_lapses_at: None,
            generators_left_to_the_game: BTreeSet::new(),
            watch: LeaveWatch::default(),
            lines: MinuteLineBudget::new(LEAVE_TEST_LINES_LOGGED_PER_MINUTE),
        });
    }
    let leave_is_hooked = hook(
        "generator leave test (a run ends when the host's rule says the players have left)",
        &calls,
        GENERATOR_LEAVE_TEST_FUNCTION,
        leave_test_detour as *const (),
        &ORIGINAL_LEAVE_TEST,
    );
    (leave_is_hooked, partner_place_is_hooked)
}

/// Sets the rule for the leave test and renews it; the rule lapses to the game's own when it is
/// not renewed. Gives a line when the rule changed.
pub fn set_leave_test_rule(
    rule: LeaveTestRule,
    generators_left_to_the_game: &BTreeSet<u32>,
) -> Option<String> {
    let mut guard = LEAVE.lock().ok()?;
    let state = guard.as_mut()?;
    let rule = if rule == LeaveTestRule::EitherPlayerKeepsTheRun && !state.partner_place_is_hooked {
        LeaveTestRule::TheGamesOwn
    } else {
        rule
    };
    state.rule_lapses_at =
        Some(Instant::now() + Duration::from_millis(LEAVE_TEST_RULE_LAPSES_AFTER_MS));
    if state.generators_left_to_the_game != *generators_left_to_the_game {
        state.generators_left_to_the_game = generators_left_to_the_game.clone();
    }
    if state.rule == rule {
        return None;
    }
    state.rule = rule;
    state.watch.forget();
    Some(format!(
        "enemies: leave test of the generators: {}",
        rule.label()
    ))
}

#[expect(
    unsafe_code,
    reason = "calls the original leave test through the trampoline MinHook built"
)]
extern "system" fn leave_test_detour(area: usize) -> u8 {
    let original = ORIGINAL_LEAVE_TEST.load(Ordering::Acquire);
    if original.is_null() {
        return PLAYERS_HAVE_NOT_LEFT;
    }
    // SAFETY: the trampoline continues into the original function.
    let original = unsafe { core::mem::transmute::<*mut c_void, LeaveTest>(original) };
    // SAFETY: the area is the one received.
    let own = unsafe { original(area) };
    let Some(asked) = asked_leave_of(area) else {
        return own;
    };
    let partner_left = if asked.rule == LeaveTestRule::EitherPlayerKeepsTheRun && own != 0 {
        partner_place().map(|place| {
            PLACE_FOR_THIS_AREA_TEST.set(Some(place));
            // SAFETY: the area is the one received; on this thread the player place hook gives the
            // partner body's place for this one call.
            let left = unsafe { original(area) };
            PLACE_FOR_THIS_AREA_TEST.set(None);
            left != 0
        })
    } else {
        None
    };
    let (left, outcome) = asked.rule.answer(own != 0, partner_left);
    note(asked.generator, outcome);
    u8::from(left)
}

#[expect(
    unsafe_code,
    reason = "calls the original player place through the trampoline MinHook built, or writes the place asked for into the game's own buffer"
)]
extern "system" fn player_place_detour(out: *mut [f32; 4]) -> *mut [f32; 4] {
    if let Some(place) = PLACE_FOR_THIS_AREA_TEST.get()
        && !out.is_null()
    {
        // SAFETY: the game passes a buffer for one place; the original writes the same 16 bytes.
        unsafe { out.write_unaligned(place) };
        return out;
    }
    let original = ORIGINAL_PLAYER_PLACE.load(Ordering::Acquire);
    if original.is_null() {
        return out;
    }
    // SAFETY: the trampoline continues into the original function with the buffer received.
    unsafe { core::mem::transmute::<*mut c_void, PlayerPlace>(original)(out) }
}

fn start_is_the_noted_one(calls: &GameCalls, function: GameAddress, noted_start: &[u8]) -> bool {
    let matches = calls
        .read_bytes(function, noted_start.len())
        .is_ok_and(|found| code_starts_with(&found, noted_start));
    if !matches {
        log_line(&format!(
            "enemies: the function at {:#X} does not start as noted, it is not hooked and the game's own leave test stays",
            calls.address_value(function)
        ));
    }
    matches
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
        nc_hook::detour::hook_address(
            label,
            calls.address_value(function) as usize,
            replacement as *mut c_void,
            original,
        )
    }
}

fn asked_leave_of(area: usize) -> Option<AskedLeave> {
    let guard = LEAVE.try_lock().ok()?;
    let state = guard.as_ref()?;
    if state.rule == LeaveTestRule::TheGamesOwn
        || state
            .rule_lapses_at
            .is_none_or(|lapses_at| Instant::now() >= lapses_at)
    {
        return None;
    }
    // Other actions call the same test on an area of their own: only a generator's is answered.
    let generator = area.checked_sub(GENERATOR_LEAVE_TEST_AREA_OFFSET)?;
    let vftable = state.calls.read_u64_at(generator as u64).ok()?;
    if vftable != state.calls.address_value(ENEMY_GENERATOR_VFTABLE) {
        return None;
    }
    let generator = state
        .calls
        .read_u32_at((generator + GENERATOR_ACTION_ID_OFFSET) as u64)
        .ok()?;
    if state.generators_left_to_the_game.contains(&generator) {
        return None;
    }
    Some(AskedLeave {
        rule: state.rule,
        generator,
    })
}

fn partner_place() -> Option<[f32; 4]> {
    let (calls, body) = guarded_slot_body()?;
    calls.body_position(body)
}

fn note(generator: u32, outcome: LeaveOutcome) {
    let Ok(mut guard) = LEAVE.try_lock() else {
        return;
    };
    let Some(state) = guard.as_mut() else {
        return;
    };
    let Some(line) = state.watch.note(generator, outcome) else {
        return;
    };
    log_owned_within(&mut state.lines, "enemies: leave test lines", line);
}
