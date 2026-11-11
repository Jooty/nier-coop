use core::ffi::c_void;
use core::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU8, Ordering};

use automata_game::address::{
    LOAD_DONE_REQUEST_VALUE, LOADING_SCREEN_ANSWER_VALUE,
    LOADING_SCREEN_STILL_LOADING_TEST_FUNCTION, SCENE_LOADER_STAGE_VALUE, SCENE_STATE_VALUE,
};
use automata_game::exe::ExeVariant;
use automata_game::load_gate::{
    BootLoadEndAction, BootLoadPass, LOAD_DONE_NOT_REQUESTED, LOADING_SCREEN_ANSWERED,
    LOADING_SCREEN_NOT_ANSWERED, LoaderReading, STILL_LOADING_TEST_CODE_CHECKED_LEN,
    boot_load_end_action, still_loading_test_code_is_the_noted_one,
};
use nc_hook::diagnostics::log_line;

use crate::game_calls::GameCalls;

const NOT_YET: u8 = 0;
const ANSWER_LENT: u8 = 1;
const PASSED: u8 = 2;
const THE_GAME_SAYS_STILL_LOADING: u64 = 1;

static ORIGINAL_STILL_LOADING_TEST: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static SCREEN_WANTED_KEPT: AtomicBool = AtomicBool::new(false);
static BOOT_LOAD_PASS: AtomicU8 = AtomicU8::new(NOT_YET);

type StillLoadingTest = unsafe extern "system" fn(*mut c_void) -> u64;

/// Hooks the test by which the game's loading screen decides whether a load is still going. True
/// when the test of this exe is the noted one and the hook is in place; otherwise nothing is
/// touched. The scene loader must be hooked too for the boot's load to end behind its screen.
pub fn install_boot_screen_keep(calls: GameCalls, variant: ExeVariant) -> bool {
    let code_is_the_noted_one = calls
        .read_bytes(
            LOADING_SCREEN_STILL_LOADING_TEST_FUNCTION,
            STILL_LOADING_TEST_CODE_CHECKED_LEN,
        )
        .is_ok_and(|code| {
            still_loading_test_code_is_the_noted_one(
                &code,
                LOADING_SCREEN_STILL_LOADING_TEST_FUNCTION.rva(variant),
                SCENE_STATE_VALUE.rva(variant),
            )
        });
    if !code_is_the_noted_one {
        log_line(
            "start: the loading screen's own test for a running load is not the noted one on this exe, the boot's loading screen is not kept",
        );
        return false;
    }
    #[expect(
        unsafe_code,
        reason = "hooks the loading screen's test; the replacement has the signature of the original"
    )]
    // SAFETY: the address is the test of the verified build and its code was just checked; it
    // takes the loading screen and gives one value back, like the replacement.
    unsafe {
        nc_hook::detour::hook_address(
            "loading screen's test for a running load (the boot's screen can be kept)",
            calls.address_value(LOADING_SCREEN_STILL_LOADING_TEST_FUNCTION) as usize,
            still_loading_test_detour as *const () as *mut c_void,
            &ORIGINAL_STILL_LOADING_TEST,
        )
    }
}

/// While wanted, the first load that ends does so behind its loading screen, and the screen stays
/// up as if that load were still going.
pub fn set_boot_screen_kept(wanted: bool) {
    SCREEN_WANTED_KEPT.store(wanted, Ordering::Release);
}

pub fn boot_load_passed_behind_its_screen() -> bool {
    BOOT_LOAD_PASS.load(Ordering::Acquire) != NOT_YET
}

/// Run after every step of the scene loader. True when the step was the end of the boot's load
/// and it was dealt with here.
pub(super) fn after_a_loader_step_of_the_boot(calls: &GameCalls, loader_finished: bool) -> bool {
    let pass = match BOOT_LOAD_PASS.load(Ordering::Acquire) {
        NOT_YET => BootLoadPass::NotYet,
        ANSWER_LENT => BootLoadPass::AnswerLent,
        _ => return false,
    };
    let reading = LoaderReading {
        stage: calls.read_i32(SCENE_LOADER_STAGE_VALUE).ok(),
        load_done_request: calls.read_i32(LOAD_DONE_REQUEST_VALUE).ok(),
        loading_screen_answer: calls.read_i32(LOADING_SCREEN_ANSWER_VALUE).ok(),
        loader_finished,
    };
    let wanted = SCREEN_WANTED_KEPT.load(Ordering::Acquire);
    match boot_load_end_action(pass, wanted, &reading) {
        BootLoadEndAction::Nothing => false,
        BootLoadEndAction::PassTheLoadKeepTheScreen => {
            // Both writes land before the loading screen runs this frame: it never sees the
            // request, and the loader finds its answer on its next step.
            if calls
                .write_i32(LOAD_DONE_REQUEST_VALUE, LOAD_DONE_NOT_REQUESTED)
                .is_err()
            {
                return false;
            }
            let _ = calls.write_i32(LOADING_SCREEN_ANSWER_VALUE, LOADING_SCREEN_ANSWERED);
            BOOT_LOAD_PASS.store(ANSWER_LENT, Ordering::Release);
            log_line(
                "start: the boot's load is done; the game goes on behind its loading screen, which stays",
            );
            true
        }
        BootLoadEndAction::TakeTheAnswerBack => {
            let _ = calls.write_i32(LOADING_SCREEN_ANSWER_VALUE, LOADING_SCREEN_NOT_ANSWERED);
            BOOT_LOAD_PASS.store(PASSED, Ordering::Release);
            true
        }
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original test through the trampoline MinHook built"
)]
extern "system" fn still_loading_test_detour(loading_screen: *mut c_void) -> u64 {
    let original = ORIGINAL_STILL_LOADING_TEST.load(Ordering::Acquire);
    if original.is_null() {
        return THE_GAME_SAYS_STILL_LOADING;
    }
    // SAFETY: the trampoline continues into the original test, which takes the loading screen.
    let still_loading =
        unsafe { core::mem::transmute::<*mut c_void, StillLoadingTest>(original)(loading_screen) };
    let kept = SCREEN_WANTED_KEPT.load(Ordering::Acquire)
        && BOOT_LOAD_PASS.load(Ordering::Acquire) != NOT_YET;
    if kept {
        THE_GAME_SAYS_STILL_LOADING
    } else {
        still_loading
    }
}
