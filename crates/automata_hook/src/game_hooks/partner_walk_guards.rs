use core::ffi::c_void;
use core::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::{Mutex, OnceLock};

use automata_game::address::{
    DESTROY_BUDDY_FUNCTION, GameAddress, PARTNER_WALK_IN_STATE_3B_FUNCTION,
    SET_BUDDY_AI_TYPE_FUNCTION,
};
use automata_game::partner::{
    BODY_PLAYER_CONTROLLED_OFFSET, HeldBodies, PARTNER_WALK_IN_STATE_3B_FUNCTION_START,
    SET_BUDDY_AI_TYPE_FUNCTION_START, code_starts_with,
};
use nt_hook::diagnostics::log_line;

use crate::game_calls::GameCalls;

static ORIGINAL_DESTROY_BUDDY: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_SET_BUDDY_AI_TYPE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_PARTNER_WALK_IN_STATE_3B: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static GUARD_CALLS: OnceLock<GameCalls> = OnceLock::new();
static HELD_BODIES: Mutex<HeldBodies> = Mutex::new(HeldBodies::new());
static FIRST_GUARDED_REMOVAL_LOGGED: AtomicBool = AtomicBool::new(false);
static FIRST_GUARDED_AI_TYPE_CALL_LOGGED: AtomicBool = AtomicBool::new(false);
static FIRST_GUARDED_STATE_3B_WALK_LOGGED: AtomicBool = AtomicBool::new(false);

type DestroyBuddy = unsafe extern "system" fn(usize);
type SetBuddyAiType = unsafe extern "system" fn(usize, i32) -> usize;
type PartnerWalkInState3B = unsafe extern "system" fn(usize) -> usize;

/// Guards the three game functions that walk from a body to the hero's partner until one is
/// not player controlled; with the held body's player flag at 1 that walk never ends.
pub fn install_partner_walk_guards(calls: GameCalls) {
    if GUARD_CALLS.set(calls).is_err() {
        return;
    }
    let address = calls.address_value(DESTROY_BUDDY_FUNCTION) as usize;
    #[expect(
        unsafe_code,
        reason = "hooks the game's partner removal; the replacement has the signature of the original"
    )]
    // SAFETY: the address is destroyBuddy of the verified build, one pointer argument and no
    // result, and the replacement forwards that argument to the original.
    unsafe {
        nt_hook::detour::hook_address(
            "partner removal (guard for the held body)",
            address,
            destroy_buddy_detour as *const () as *mut c_void,
            &ORIGINAL_DESTROY_BUDDY,
        );
    }
    if start_is_the_noted_one(
        &calls,
        SET_BUDDY_AI_TYPE_FUNCTION,
        &SET_BUDDY_AI_TYPE_FUNCTION_START,
        "the partner AI type setter",
    ) {
        #[expect(
            unsafe_code,
            reason = "hooks the game's partner AI type setter; the replacement has the signature of the original"
        )]
        // SAFETY: the address is setBuddyAiType of the verified build and its start was just
        // checked; the replacement forwards both arguments and hands the result back.
        unsafe {
            nt_hook::detour::hook_address(
                "partner AI type setter (guard for the held body)",
                calls.address_value(SET_BUDDY_AI_TYPE_FUNCTION) as usize,
                set_buddy_ai_type_detour as *const () as *mut c_void,
                &ORIGINAL_SET_BUDDY_AI_TYPE,
            );
        }
    }
    if start_is_the_noted_one(
        &calls,
        PARTNER_WALK_IN_STATE_3B_FUNCTION,
        &PARTNER_WALK_IN_STATE_3B_FUNCTION_START,
        "the partner walk of state 0x3B",
    ) {
        #[expect(
            unsafe_code,
            reason = "hooks the partner walk of state 0x3B; the replacement has the signature of the original"
        )]
        // SAFETY: the address is the one function the state 0x3B handler calls with the body
        // and its start was just checked; the replacement forwards the argument and the result.
        unsafe {
            nt_hook::detour::hook_address(
                "partner walk of state 0x3B (guard for the held body)",
                calls.address_value(PARTNER_WALK_IN_STATE_3B_FUNCTION) as usize,
                partner_walk_in_state_3b_detour as *const () as *mut c_void,
                &ORIGINAL_PARTNER_WALK_IN_STATE_3B,
            );
        }
    }
}

pub fn hold_body(handle: u32, slot: u8, is_slot_body: bool) -> bool {
    HELD_BODIES
        .lock()
        .is_ok_and(|mut held| held.add(handle, slot, is_slot_body))
}

pub fn release_body(handle: u32) {
    if let Ok(mut held) = HELD_BODIES.lock() {
        held.remove(handle);
    }
}

pub fn is_held_body(handle: u32) -> bool {
    HELD_BODIES.lock().is_ok_and(|held| held.holds(handle))
}

/// Whether this body object is one of the held bodies, resolved by its handle now; game thread only.
pub fn body_is_held(body: usize) -> bool {
    let Some(calls) = GUARD_CALLS.get() else {
        return false;
    };
    if HELD_BODIES.lock().is_ok_and(|held| held.is_empty()) {
        return false;
    }
    calls.entity_handle_of_body(body).is_some_and(is_held_body)
}

/// The companion slot body among the held ones, resolved from its handle now; game thread only.
pub fn guarded_slot_body() -> Option<(GameCalls, usize)> {
    let handle = HELD_BODIES.lock().ok()?.slot_body()?;
    let calls = GUARD_CALLS.get()?;
    let body = calls.player_body_of_handle(handle);
    (body != 0).then_some((*calls, body))
}

#[expect(
    unsafe_code,
    reason = "calls the original partner removal through the trampoline MinHook built"
)]
extern "system" fn destroy_buddy_detour(body: usize) {
    clear_the_player_flag_of_the_held_body(
        "the game removes a partner",
        &FIRST_GUARDED_REMOVAL_LOGGED,
    );
    let original = ORIGINAL_DESTROY_BUDDY.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original function with the argument received.
    unsafe { core::mem::transmute::<*mut c_void, DestroyBuddy>(original)(body) };
}

#[expect(
    unsafe_code,
    reason = "calls the original AI type setter through the trampoline MinHook built"
)]
extern "system" fn set_buddy_ai_type_detour(body: usize, ai_type: i32) -> usize {
    clear_the_player_flag_of_the_held_body(
        "the game sets the partner's AI type",
        &FIRST_GUARDED_AI_TYPE_CALL_LOGGED,
    );
    let original = ORIGINAL_SET_BUDDY_AI_TYPE.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    // SAFETY: the trampoline continues into the original function with the arguments received.
    unsafe { core::mem::transmute::<*mut c_void, SetBuddyAiType>(original)(body, ai_type) }
}

#[expect(
    unsafe_code,
    reason = "calls the original partner walk through the trampoline MinHook built"
)]
extern "system" fn partner_walk_in_state_3b_detour(body: usize) -> usize {
    clear_the_player_flag_of_the_held_body(
        "state 0x3B walks to the partner",
        &FIRST_GUARDED_STATE_3B_WALK_LOGGED,
    );
    let original = ORIGINAL_PARTNER_WALK_IN_STATE_3B.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    // SAFETY: the trampoline continues into the original function with the argument received.
    unsafe { core::mem::transmute::<*mut c_void, PartnerWalkInState3B>(original)(body) }
}

fn start_is_the_noted_one(
    calls: &GameCalls,
    function: GameAddress,
    noted_start: &[u8],
    what: &str,
) -> bool {
    let matches = calls
        .read_bytes(function, noted_start.len())
        .is_ok_and(|found| code_starts_with(&found, noted_start));
    if !matches {
        log_line(&format!(
            "partner: the start of {what} is not the noted one, it is not guarded (the game can hang if it runs while a body is held)"
        ));
    }
    matches
}

fn clear_the_player_flag_of_the_held_body(what_the_game_does: &str, first_logged: &AtomicBool) {
    let Some((calls, body)) = guarded_slot_body() else {
        return;
    };
    // The game walks to the hero's partner until a body has no player flag; with the flag
    // held at 1 on the partner that walk would never end. The next hold puts the flag back.
    let cleared = calls.write_i32_at(body as u64 + u64::from(BODY_PLAYER_CONTROLLED_OFFSET), 0);
    if !first_logged.swap(true, Ordering::Relaxed) {
        log_line(&format!(
            "partner: {what_the_game_does} while a body is held; cleared the held body's player flag first ({})",
            if cleared.is_ok() {
                "written"
            } else {
                "the write failed"
            }
        ));
    }
}
