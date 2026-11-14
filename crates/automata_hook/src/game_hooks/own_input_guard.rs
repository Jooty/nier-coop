use core::ffi::c_void;
use core::ptr;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

use automata_game::address::{
    BODY_UPDATE_FUNCTION, BODY_UPDATE_INPUT_MODE_COPY, KEYBOARD_AND_MOUSE_INPUT_MODE_VALUE,
};
use automata_game::exe::ExeVariant;
use automata_game::partner::{
    BODY_UPDATE_FUNCTION_START, BODY_UPDATE_INPUT_MODE_COPY_LEN, InputModeKept,
    body_update_copies_the_input_mode, code_starts_with,
};
use nt_hook::diagnostics::log_line;

use crate::game_calls::GameCalls;
use crate::game_hooks::held_body_update_mark::while_a_held_body_update_runs;
use crate::game_hooks::partner_walk_guards::body_is_held;

static ORIGINAL_BODY_UPDATE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static OWN_INPUT_CALLS: OnceLock<GameCalls> = OnceLock::new();
static FIRST_PAD_MODE_UPDATE_LOGGED: AtomicBool = AtomicBool::new(false);

type BodyUpdate = unsafe extern "system" fn(usize) -> usize;

/// Keeps the local keyboard and mouse from the held body: its update runs as if the game were in
/// pad mode. False, with nothing hooked, when the code differs from the noted one.
pub fn install_own_input_guard(calls: GameCalls, variant: ExeVariant) -> bool {
    let start_matches = calls
        .read_bytes(BODY_UPDATE_FUNCTION, BODY_UPDATE_FUNCTION_START.len())
        .is_ok_and(|found| code_starts_with(&found, &BODY_UPDATE_FUNCTION_START));
    let copy_matches = calls
        .read_bytes(BODY_UPDATE_INPUT_MODE_COPY, BODY_UPDATE_INPUT_MODE_COPY_LEN)
        .ok()
        .and_then(|bytes| <[u8; BODY_UPDATE_INPUT_MODE_COPY_LEN]>::try_from(bytes).ok())
        .is_some_and(|found| {
            body_update_copies_the_input_mode(
                &found,
                BODY_UPDATE_INPUT_MODE_COPY.rva(variant),
                KEYBOARD_AND_MOUSE_INPUT_MODE_VALUE.rva(variant),
            )
        });
    let hooked = start_matches && copy_matches && OWN_INPUT_CALLS.set(calls).is_ok() && {
        #[expect(
            unsafe_code,
            reason = "hooks the body's update; the replacement has the signature of the original"
        )]
        // SAFETY: the address is the body update (vtable slot 7) of the verified build, its
        // start and its input mode copy were just checked, and the replacement forwards the
        // one argument and the result.
        unsafe {
            nt_hook::detour::hook_address(
                "player body update (keeps the local keyboard and mouse from the partner body)",
                calls.address_value(BODY_UPDATE_FUNCTION) as usize,
                body_update_detour as *const () as *mut c_void,
                &ORIGINAL_BODY_UPDATE,
            )
        }
    };
    if !hooked {
        log_line(&format!(
            "own input: the body's update is not the noted one or its hook failed (start bytes match: {start_matches}, input mode copy matches: {copy_matches}); with keyboard and mouse the partner body also acts on the local player's keys"
        ));
    }
    hooked
}

#[expect(
    unsafe_code,
    reason = "calls the original body update through the trampoline MinHook built"
)]
extern "system" fn body_update_detour(body: usize) -> usize {
    let original = ORIGINAL_BODY_UPDATE.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    let held = body_is_held(body);
    let kept = held.then(|| pad_mode_for_the_held_body(body)).flatten();
    // SAFETY: the trampoline continues into the original function with the argument received.
    let update = || unsafe { core::mem::transmute::<*mut c_void, BodyUpdate>(original)(body) };
    let result = if held {
        while_a_held_body_update_runs(update)
    } else {
        update()
    };
    if let Some((calls, kept)) = kept {
        put_the_input_mode_back(&calls, kept);
    }
    result
}

fn pad_mode_for_the_held_body(body: usize) -> Option<(GameCalls, InputModeKept)> {
    let calls = *OWN_INPUT_CALLS.get()?;
    let mode = calls.read_u32(KEYBOARD_AND_MOUSE_INPUT_MODE_VALUE).ok()?;
    let kept = InputModeKept::for_body_update(body, Some(body), mode)?;
    // The update copies this global into the body's keyboard and mouse enables at its start.
    calls
        .write_u32(
            KEYBOARD_AND_MOUSE_INPUT_MODE_VALUE,
            kept.mode_during_the_update(),
        )
        .ok()?;
    if !FIRST_PAD_MODE_UPDATE_LOGGED.swap(true, Ordering::Relaxed) {
        log_line(&format!(
            "own input: the game is in keyboard and mouse mode ({mode}); the partner body's update sees pad mode, so the game itself turns that body's keyboard and mouse sources off (that the body then acts on the other player's input only is a guess until run)"
        ));
    }
    Some((calls, kept))
}

fn put_the_input_mode_back(calls: &GameCalls, kept: InputModeKept) {
    let put_back = calls
        .read_u32(KEYBOARD_AND_MOUSE_INPUT_MODE_VALUE)
        .ok()
        .and_then(|mode_now| kept.mode_to_put_back(mode_now));
    if let Some(mode) = put_back {
        let _ = calls.write_u32(KEYBOARD_AND_MOUSE_INPUT_MODE_VALUE, mode);
    }
}
