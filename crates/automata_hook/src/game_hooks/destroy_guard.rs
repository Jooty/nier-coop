use core::ffi::c_void;
use core::ptr;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, Ordering};

use automata_game::address::DESTROY_BUDDY_FUNCTION;
use automata_game::partner::BODY_PLAYER_CONTROLLED_OFFSET;
use nc_hook::diagnostics::log_line;

use crate::game_calls::GameCalls;

const NO_HELD_BODY: u32 = 0;

static ORIGINAL_DESTROY_BUDDY: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static GUARD_CALLS: OnceLock<GameCalls> = OnceLock::new();
static HELD_BODY_HANDLE: AtomicU32 = AtomicU32::new(NO_HELD_BODY);
static FIRST_GUARDED_CALL_LOGGED: AtomicBool = AtomicBool::new(false);

type DestroyBuddy = unsafe extern "system" fn(usize);

pub fn install_destroy_guard(calls: GameCalls) {
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
        nc_hook::detour::hook_address(
            "partner removal (guard for the held body)",
            address,
            destroy_buddy_detour as *const () as *mut c_void,
            &ORIGINAL_DESTROY_BUDDY,
        );
    }
}

pub fn guard_held_body(handle: Option<u32>) {
    HELD_BODY_HANDLE.store(handle.unwrap_or(NO_HELD_BODY), Ordering::Release);
}

#[expect(
    unsafe_code,
    reason = "calls the original partner removal through the trampoline MinHook built"
)]
extern "system" fn destroy_buddy_detour(body: usize) {
    clear_the_player_flag_of_the_held_body();
    let original = ORIGINAL_DESTROY_BUDDY.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original function with the argument received.
    unsafe { core::mem::transmute::<*mut c_void, DestroyBuddy>(original)(body) };
}

fn clear_the_player_flag_of_the_held_body() {
    let handle = HELD_BODY_HANDLE.load(Ordering::Acquire);
    let Some(calls) = GUARD_CALLS.get() else {
        return;
    };
    if handle == NO_HELD_BODY {
        return;
    }
    let body = calls.player_body_of_handle(handle);
    if body == 0 {
        return;
    }
    // The game walks from the hero to its partner until a body has no player flag; with the flag
    // held at 1 on the partner that walk would never end.
    let cleared = calls.write_i32_at(body as u64 + u64::from(BODY_PLAYER_CONTROLLED_OFFSET), 0);
    if !FIRST_GUARDED_CALL_LOGGED.swap(true, Ordering::Relaxed) {
        log_line(&format!(
            "partner: the game removes a partner while a body is held; cleared the held body's player flag first ({})",
            if cleared.is_ok() {
                "written"
            } else {
                "the write failed"
            }
        ));
    }
}
