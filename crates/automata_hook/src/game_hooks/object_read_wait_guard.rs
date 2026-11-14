use core::ffi::c_void;
use core::ptr;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, Ordering};

use automata_game::address::WAIT_FOR_OBJECT_READ_FUNCTION;
use automata_game::partner::{WAIT_FOR_OBJECT_READ_FUNCTION_START, code_starts_with};
use nt_hook::diagnostics::log_line;

use crate::game_calls::GameCalls;

static ORIGINAL_WAIT_FOR_OBJECT_READ: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static GUARD_CALLS: OnceLock<GameCalls> = OnceLock::new();
static WAITS_REFUSED: AtomicU32 = AtomicU32::new(0);
static FIRST_REFUSED_WAIT_LOGGED: AtomicBool = AtomicBool::new(false);

type WaitForObjectRead = unsafe extern "system" fn();

/// Guards the game's wait for an object file: outside a task of the scene scheduler that wait
/// reads a null task and the game dies. True when the guard is in place.
pub fn install_object_read_wait_guard(calls: GameCalls) -> bool {
    if GUARD_CALLS.set(calls).is_err() {
        return false;
    }
    let start_is_the_noted_one = calls
        .read_bytes(
            WAIT_FOR_OBJECT_READ_FUNCTION,
            WAIT_FOR_OBJECT_READ_FUNCTION_START.len(),
        )
        .is_ok_and(|found| code_starts_with(&found, &WAIT_FOR_OBJECT_READ_FUNCTION_START));
    if !start_is_the_noted_one {
        log_line(
            "partner: the start of the game's wait for an object read is not the noted one, it is not guarded; no partner body will be spawned",
        );
        return false;
    }
    #[expect(
        unsafe_code,
        reason = "hooks the game's wait for an object read; the replacement has the signature of the original"
    )]
    // SAFETY: the address is the wait of the verified build and its start was just checked; it
    // takes no arguments and gives no result, like the replacement.
    unsafe {
        nt_hook::detour::hook_address(
            "wait for an object read (guard for a wait outside a task)",
            calls.address_value(WAIT_FOR_OBJECT_READ_FUNCTION) as usize,
            wait_for_object_read_detour as *const () as *mut c_void,
            &ORIGINAL_WAIT_FOR_OBJECT_READ,
        )
    }
}

pub fn object_read_waits_refused() -> u32 {
    WAITS_REFUSED.load(Ordering::Acquire)
}

#[expect(
    unsafe_code,
    reason = "calls the original wait through the trampoline MinHook built"
)]
extern "system" fn wait_for_object_read_detour() {
    let inside_a_task = GUARD_CALLS
        .get()
        .is_some_and(GameCalls::runs_inside_a_scene_task);
    let original = ORIGINAL_WAIT_FOR_OBJECT_READ.load(Ordering::Acquire);
    if !inside_a_task || original.is_null() {
        // Returning makes the caller find the object still not loaded and fail its spawn.
        WAITS_REFUSED.fetch_add(1, Ordering::AcqRel);
        if !FIRST_REFUSED_WAIT_LOGGED.swap(true, Ordering::AcqRel) {
            log_line(
                "partner: the game wanted to wait for an object read outside a task (it would have read a null task and died); the wait was refused, so that spawn fails instead",
            );
        }
        return;
    }
    // SAFETY: the trampoline continues into the original function, which takes no arguments.
    unsafe { core::mem::transmute::<*mut c_void, WaitForObjectRead>(original)() };
}
