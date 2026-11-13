use core::ffi::c_void;
use core::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

use automata_game::address::GAME_FOCUS_LOST_TEST_FUNCTION;
use automata_game::focus::{GAME_FOCUS_LOST_TEST_FUNCTION_START, focus_test_answer};
use automata_game::partner::code_starts_with;
use nc_hook::diagnostics::log_line;

use crate::game_calls::GameCalls;

static ORIGINAL_FOCUS_LOST_TEST: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static SESSION_LINKED: AtomicBool = AtomicBool::new(false);
static FIRST_PASS_OVER_LOGGED: AtomicBool = AtomicBool::new(false);
static GAME_SAYS_FOCUS_LOST: AtomicBool = AtomicBool::new(false);

type FocusLostTest = unsafe extern "system" fn() -> usize;

/// Game thread: whether a co-op session is linked now, for the focus test's detour.
pub fn set_session_linked(linked: bool) {
    SESSION_LINKED.store(linked, Ordering::Relaxed);
}

/// The game's own answer the last time it asked whether its window lost focus.
pub fn game_says_its_window_lost_focus() -> bool {
    GAME_SAYS_FOCUS_LOST.load(Ordering::Relaxed)
}

/// Hooks the test the game's pause opener asks; false, with nothing hooked, when its start differs.
pub fn install_focus_pass(calls: GameCalls) -> bool {
    let start_matches = calls
        .read_bytes(
            GAME_FOCUS_LOST_TEST_FUNCTION,
            GAME_FOCUS_LOST_TEST_FUNCTION_START.len(),
        )
        .is_ok_and(|found| code_starts_with(&found, &GAME_FOCUS_LOST_TEST_FUNCTION_START));
    let hooked = start_matches
        && {
            #[expect(
                unsafe_code,
                reason = "hooks the game's focus test; the replacement has the signature of the original"
            )]
            // SAFETY: the address is the focus test of the verified build and its start was just
            // checked; the replacement takes no arguments and returns the original's kind of answer.
            unsafe {
                nc_hook::detour::hook_address(
                    "game focus test (keeps a linked game running without focus)",
                    calls.address_value(GAME_FOCUS_LOST_TEST_FUNCTION) as usize,
                    focus_lost_test_detour as *const () as *mut c_void,
                    &ORIGINAL_FOCUS_LOST_TEST,
                )
            }
        };
    if !hooked {
        log_line(
            "focus: the game's focus test is not hooked, a copy without focus will show the game's pause",
        );
    }
    hooked
}

#[expect(
    unsafe_code,
    reason = "calls the original focus test through the trampoline MinHook built"
)]
extern "system" fn focus_lost_test_detour() -> usize {
    let original = ORIGINAL_FOCUS_LOST_TEST.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    // SAFETY: the trampoline continues into the original function, which takes no arguments.
    let game_says = unsafe { core::mem::transmute::<*mut c_void, FocusLostTest>(original)() };
    GAME_SAYS_FOCUS_LOST.store(game_says != 0, Ordering::Relaxed);
    let answer = focus_test_answer(game_says != 0, SESSION_LINKED.load(Ordering::Relaxed));
    if answer.passed_over && !FIRST_PASS_OVER_LOGGED.swap(true, Ordering::Relaxed) {
        log_line(
            "focus: the window lost focus or was minimised while a session is linked; the game is told it has focus, so its pause does not open (input stays zeroed by the game's own inactive path)",
        );
    }
    usize::from(answer.focus_lost)
}
