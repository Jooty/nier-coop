use core::cell::Cell;
use core::ffi::c_void;
use core::ptr;
use std::collections::HashMap;
use std::sync::atomic::{AtomicPtr, AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use automata_game::address::{
    ENEMY_STATE_START_FUNCTION, ENEMY_TARGET_SETTER_FUNCTION, GameAddress,
};
use automata_game::enemies::EMPTY_ENTITY_HANDLES;
use automata_game::enemy_lead::{
    ENEMY_STATE_START_FUNCTION_START, ENEMY_TARGET_SETTER_FUNCTION_START, HookedStarts,
};
use automata_game::partner::code_starts_with;
use nt_hook::diagnostics::log_line;

use crate::game_calls::GameCalls;
use crate::game_hooks::partner_walk_guards::guarded_slot_body;

static ORIGINAL_STATE_START: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_TARGET_SETTER: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static HOLD_CALLS: OnceLock<GameCalls> = OnceLock::new();
static KEPT: Mutex<Option<HashMap<usize, Kept>>> = Mutex::new(None);
static OWN_STARTS_HELD: AtomicU32 = AtomicU32::new(0);
static TARGETS_REPLACED: AtomicU32 = AtomicU32::new(0);

thread_local! {
    static A_LEAD_START_IS_RUNNING: Cell<bool> = const { Cell::new(false) };
}

type EnemyStateStart = unsafe extern "system" fn(usize, i32, u32, u32, u32);
type EnemyTargetSetter = unsafe extern "system" fn(usize, *const u32) -> usize;

/// A shared enemy as the enemies driver last set it up for the two hooks, by its body.
/// `held_until` is the guest's: until then the enemy's own state starts are refused. The host
/// leaves it empty and its starts are counted. `target` replaces a player as the enemy's target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LedBody {
    pub handle: u32,
    pub held_until: Option<Instant>,
    pub target: Option<u32>,
    pub player_handles: [u32; 2],
}

#[derive(Debug, Clone, Copy)]
struct Kept {
    led: LedBody,
    starts: Option<HookedStarts>,
}

/// What the hooks did since the last call: own state starts held back on the guest and player
/// targets replaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HoldCounts {
    pub own_starts_held: u32,
    pub targets_replaced: u32,
}

/// Hooks the enemies' state start and target setter; each is false, and that part left to the
/// game, when its function start differs from the noted one.
pub fn install_enemy_ai_hold(calls: GameCalls) -> (bool, bool) {
    let _ = HOLD_CALLS.set(calls);
    let states = start_is_the_noted_one(
        &calls,
        ENEMY_STATE_START_FUNCTION,
        &ENEMY_STATE_START_FUNCTION_START,
    ) && {
        #[expect(
            unsafe_code,
            reason = "hooks the enemies' state start; the replacement has the signature of the original"
        )]
        // SAFETY: the address is slot 18 of the enemy base vtable in the verified build and its
        // start was just checked; the replacement forwards all five arguments or none.
        unsafe {
            nt_hook::detour::hook_address(
                "enemy state start (the host counts starts, the guest holds its own)",
                calls.address_value(ENEMY_STATE_START_FUNCTION) as usize,
                enemy_state_start_detour as *const () as *mut c_void,
                &ORIGINAL_STATE_START,
            )
        }
    };
    let targets = start_is_the_noted_one(
        &calls,
        ENEMY_TARGET_SETTER_FUNCTION,
        &ENEMY_TARGET_SETTER_FUNCTION_START,
    ) && {
        #[expect(
            unsafe_code,
            reason = "hooks the enemies' target setter; the replacement has the signature of the original"
        )]
        // SAFETY: the address is the enemy target setter of the verified build and its start
        // was just checked; the replacement forwards the enemy and a pointer to a live handle.
        unsafe {
            nt_hook::detour::hook_address(
                "enemy target setter (the host picks whom a shared enemy aims at)",
                calls.address_value(ENEMY_TARGET_SETTER_FUNCTION) as usize,
                enemy_target_setter_detour as *const () as *mut c_void,
                &ORIGINAL_TARGET_SETTER,
            )
        }
    };
    (states, targets)
}

/// The shared enemies the two hooks act on, by body; a body that stays with the same handle
/// keeps the starts counted for it so far.
pub fn set_led_bodies(bodies: HashMap<usize, LedBody>) {
    let Ok(mut kept) = KEPT.lock() else {
        return;
    };
    let before = kept.take().unwrap_or_default();
    let next: HashMap<usize, Kept> = bodies
        .into_iter()
        .map(|(body, led)| {
            let starts = before
                .get(&body)
                .filter(|old| old.led.handle == led.handle)
                .and_then(|old| old.starts);
            (body, Kept { led, starts })
        })
        .collect();
    *kept = (!next.is_empty()).then_some(next);
}

pub fn hooked_starts_of(body: usize) -> Option<HookedStarts> {
    KEPT.lock().ok()?.as_ref()?.get(&body)?.starts
}

/// Runs a state start the mod makes for the host's lead; the hook lets it through.
pub fn while_a_lead_start_runs<R>(start: impl FnOnce() -> R) -> R {
    let before = A_LEAD_START_IS_RUNNING.replace(true);
    let result = start();
    A_LEAD_START_IS_RUNNING.set(before);
    result
}

pub fn take_hold_counts() -> HoldCounts {
    HoldCounts {
        own_starts_held: OWN_STARTS_HELD.swap(0, Ordering::Relaxed),
        targets_replaced: TARGETS_REPLACED.swap(0, Ordering::Relaxed),
    }
}

/// The handle of the body the other player drives on this copy, resolved now; game thread only.
pub fn partner_body_handle() -> Option<u32> {
    let (calls, body) = guarded_slot_body()?;
    calls.entity_handle_of_body(body)
}

#[expect(
    unsafe_code,
    reason = "calls the original enemy state start through the trampoline MinHook built"
)]
extern "system" fn enemy_state_start_detour(enemy: usize, state: i32, a: u32, b: u32, c: u32) {
    let original = ORIGINAL_STATE_START.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    if !A_LEAD_START_IS_RUNNING.get()
        && !start_goes_on(enemy, [state, a as i32, b as i32, c as i32])
    {
        OWN_STARTS_HELD.fetch_add(1, Ordering::Relaxed);
        return;
    }
    // SAFETY: the trampoline continues into the original function with the arguments received.
    unsafe {
        core::mem::transmute::<*mut c_void, EnemyStateStart>(original)(enemy, state, a, b, c);
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original enemy target setter through the trampoline MinHook built"
)]
extern "system" fn enemy_target_setter_detour(enemy: usize, handle: *const u32) -> usize {
    let original = ORIGINAL_TARGET_SETTER.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    let setter = |handle: *const u32| {
        // SAFETY: the trampoline continues into the original function with the enemy received
        // and a pointer to a handle that lives through the call.
        unsafe { core::mem::transmute::<*mut c_void, EnemyTargetSetter>(original)(enemy, handle) }
    };
    // SAFETY: the game passes a pointer to a handle on its own stack or in an object; a null
    // pointer is never read.
    let asked = (!handle.is_null()).then(|| unsafe { handle.read_unaligned() });
    match asked.and_then(|asked| replacement_target(enemy, asked)) {
        Some(replacement) => {
            TARGETS_REPLACED.fetch_add(1, Ordering::Relaxed);
            setter(&replacement)
        }
        None => setter(handle),
    }
}

fn start_is_the_noted_one(calls: &GameCalls, function: GameAddress, noted_start: &[u8]) -> bool {
    let matches = calls
        .read_bytes(function, noted_start.len())
        .is_ok_and(|found| code_starts_with(&found, noted_start));
    if !matches {
        log_line(&format!(
            "enemies: the function at {:#X} does not start as noted, it is not hooked",
            calls.address_value(function)
        ));
    }
    matches
}

fn kept_body(enemy: usize) -> Option<Kept> {
    let calls = HOLD_CALLS.get()?;
    let kept = *KEPT.try_lock().ok()?.as_ref()?.get(&enemy)?;
    (calls.entity_handle_of_body(enemy) == Some(kept.led.handle)).then_some(kept)
}

fn start_goes_on(enemy: usize, state: [i32; 4]) -> bool {
    let Some(kept) = kept_body(enemy) else {
        return true;
    };
    let Some(held_until) = kept.led.held_until else {
        note_hosts_start(enemy, state);
        return true;
    };
    // A defeated enemy runs its own death; a hold the host has not renewed has lifted.
    let alive = HOLD_CALLS
        .get()
        .and_then(|calls| calls.read_enemy_health(enemy))
        .is_some_and(|health| health >= 1);
    !(alive && Instant::now() < held_until)
}

fn note_hosts_start(enemy: usize, state: [i32; 4]) {
    let Ok(mut kept) = KEPT.try_lock() else {
        return;
    };
    if let Some(body) = kept.as_mut().and_then(|kept| kept.get_mut(&enemy)) {
        body.starts = Some(HookedStarts {
            count: body.starts.map_or(1, |starts| starts.count.wrapping_add(1)),
            last: state,
        });
    }
}

fn replacement_target(enemy: usize, asked: u32) -> Option<u32> {
    let kept = kept_body(enemy)?;
    let target = kept.led.target?;
    let in_force = kept
        .led
        .held_until
        .is_none_or(|held_until| Instant::now() < held_until);
    let asked_a_player =
        !EMPTY_ENTITY_HANDLES.contains(&asked) && kept.led.player_handles.contains(&asked);
    (in_force && asked_a_player && asked != target).then_some(target)
}
