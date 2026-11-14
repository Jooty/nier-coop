use core::cell::Cell;
use core::ffi::c_void;
use core::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};

use automata_game::address::{
    LOAD_DONE_REQUEST_VALUE, LOADING_SCREEN_ANSWER_VALUE,
    LOADING_SCREEN_STILL_LOADING_TEST_FUNCTION, SCENE_LOADER_STAGE_VALUE, SCENE_STATE_VALUE,
    SYSTEM_DATA_STAGE_VALUE, SYSTEM_DIALOG_OPEN_FUNCTION, TITLE_SYSTEM_DATA_STEP_FUNCTION,
};
use automata_game::exe::ExeVariant;
use automata_game::load_gate::{
    BootLoadEndAction, BootLoadPass, BootScreenDialogs, LOAD_DONE_NOT_REQUESTED,
    LOADING_SCREEN_ANSWERED, LOADING_SCREEN_NOT_ANSWERED, LoaderReading,
    STILL_LOADING_TEST_CODE_CHECKED_LEN, SYSTEM_DIALOG_NOT_OPENED_HANDLE,
    SYSTEM_DIALOG_OPEN_FUNCTION_START, SystemDialogAnswer, TITLE_SYSTEM_DATA_STEP_CODE_CHECKED_LEN,
    boot_load_end_action, still_loading_test_code_is_the_noted_one,
    system_dialog_open_code_is_the_noted_one, title_system_data_step_code_is_the_noted_one,
};
use nt_hook::diagnostics::log_line;

use crate::game_calls::GameCalls;

const NOT_YET: u8 = 0;
const ANSWER_LENT: u8 = 1;
const PASSED: u8 = 2;
const THE_GAME_SAYS_STILL_LOADING: u64 = 1;

static ORIGINAL_STILL_LOADING_TEST: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static SCREEN_WANTED_KEPT: AtomicBool = AtomicBool::new(false);
static BOOT_LOAD_PASS: AtomicU8 = AtomicU8::new(NOT_YET);
static ORIGINAL_TITLE_SYSTEM_DATA_STEP: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_SYSTEM_DIALOG_OPEN: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static BAND_CALLS: OnceLock<GameCalls> = OnceLock::new();
static BOOT_SCREEN_DIALOGS: Mutex<Option<BootScreenDialogs>> = Mutex::new(None);

thread_local! {
    static INSIDE_THE_TITLE_SYSTEM_DATA_STEP: Cell<bool> = const { Cell::new(false) };
}

type StillLoadingTest = unsafe extern "system" fn(*mut c_void) -> u64;
type TitleSystemDataStep = unsafe extern "system" fn(*mut c_void);
type SystemDialogOpen =
    unsafe extern "system" fn(i32, i32, *const c_void, *const c_void, i32) -> i32;

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
        nt_hook::detour::hook_address(
            "loading screen's test for a running load (the boot's screen can be kept)",
            calls.address_value(LOADING_SCREEN_STILL_LOADING_TEST_FUNCTION) as usize,
            still_loading_test_detour as *const () as *mut c_void,
            &ORIGINAL_STILL_LOADING_TEST,
        )
    }
}

/// Hooks the title's system data step and the game's system dialog opener, so the band the step
/// shows while the system data loads is held back under the kept boot screen. True when both
/// codes are the noted ones and both hooks are in place; otherwise nothing is touched.
pub fn install_boot_screen_band_hold(calls: GameCalls, variant: ExeVariant) -> bool {
    let step_is_the_noted_one = calls
        .read_bytes(
            TITLE_SYSTEM_DATA_STEP_FUNCTION,
            TITLE_SYSTEM_DATA_STEP_CODE_CHECKED_LEN,
        )
        .is_ok_and(|code| {
            title_system_data_step_code_is_the_noted_one(
                &code,
                TITLE_SYSTEM_DATA_STEP_FUNCTION.rva(variant),
                SYSTEM_DIALOG_OPEN_FUNCTION.rva(variant),
            )
        });
    let opener_is_the_noted_one = calls
        .read_bytes(
            SYSTEM_DIALOG_OPEN_FUNCTION,
            SYSTEM_DIALOG_OPEN_FUNCTION_START.len(),
        )
        .is_ok_and(|code| system_dialog_open_code_is_the_noted_one(&code));
    if !step_is_the_noted_one || !opener_is_the_noted_one {
        log_line(
            "start: the title's system data step or the system dialog opener is not the noted code on this exe, the system data band may show over the boot's loading screen",
        );
        return false;
    }
    let _ = BAND_CALLS.set(calls);
    if let Ok(mut dialogs) = BOOT_SCREEN_DIALOGS.lock() {
        *dialogs = Some(BootScreenDialogs::default());
    }
    #[expect(
        unsafe_code,
        reason = "hooks the title's system data step; the replacement has the signature of the original"
    )]
    // SAFETY: the address is the step of the verified build and its code was just checked; it
    // takes the title object and returns nothing, like the replacement.
    let step_hooked = unsafe {
        nt_hook::detour::hook_address(
            "title's system data step (its band is held back under the boot's screen)",
            calls.address_value(TITLE_SYSTEM_DATA_STEP_FUNCTION) as usize,
            title_system_data_step_detour as *const () as *mut c_void,
            &ORIGINAL_TITLE_SYSTEM_DATA_STEP,
        )
    };
    if !step_hooked {
        return false;
    }
    #[expect(
        unsafe_code,
        reason = "hooks the system dialog opener; the replacement has the signature of the original"
    )]
    // SAFETY: the address is the opener of the verified build and its start was just checked; it
    // takes four register arguments and one on the stack and returns the dialog handle, like the
    // replacement.
    unsafe {
        nt_hook::detour::hook_address(
            "system dialog opener (the title's system data band is held back under the boot's screen)",
            calls.address_value(SYSTEM_DIALOG_OPEN_FUNCTION) as usize,
            system_dialog_open_detour as *const () as *mut c_void,
            &ORIGINAL_SYSTEM_DIALOG_OPEN,
        )
    }
}

/// While wanted, the first load that ends does so behind its loading screen, and the screen stays
/// up as if that load were still going.
pub fn set_boot_screen_kept(wanted: bool) {
    let was_kept = SCREEN_WANTED_KEPT.swap(wanted, Ordering::AcqRel);
    if was_kept
        && !wanted
        && let Ok(dialogs) = BOOT_SCREEN_DIALOGS.lock()
        && let Some(dialogs) = dialogs.as_ref()
    {
        log_line(&dialogs.summary_when_the_keep_ends());
    }
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

#[expect(
    unsafe_code,
    reason = "calls the original step through the trampoline MinHook built"
)]
extern "system" fn title_system_data_step_detour(title: *mut c_void) {
    let original = ORIGINAL_TITLE_SYSTEM_DATA_STEP.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    INSIDE_THE_TITLE_SYSTEM_DATA_STEP.with(|inside| inside.set(true));
    // SAFETY: the trampoline continues into the original step, which takes the title object.
    unsafe { core::mem::transmute::<*mut c_void, TitleSystemDataStep>(original)(title) };
    INSIDE_THE_TITLE_SYSTEM_DATA_STEP.with(|inside| inside.set(false));
}

#[expect(
    unsafe_code,
    reason = "calls the original opener through the trampoline MinHook built"
)]
extern "system" fn system_dialog_open_detour(
    first: i32,
    second: i32,
    text: *const c_void,
    fourth: *const c_void,
    kind: i32,
) -> i32 {
    let original = ORIGINAL_SYSTEM_DIALOG_OPEN.load(Ordering::Acquire);
    if original.is_null() {
        return SYSTEM_DIALOG_NOT_OPENED_HANDLE;
    }
    if system_dialog_answer() == SystemDialogAnswer::HoldItBack {
        return SYSTEM_DIALOG_NOT_OPENED_HANDLE;
    }
    // SAFETY: the trampoline continues into the original opener with the arguments it was given.
    unsafe {
        core::mem::transmute::<*mut c_void, SystemDialogOpen>(original)(
            first, second, text, fourth, kind,
        )
    }
}

fn system_dialog_answer() -> SystemDialogAnswer {
    let screen_kept = SCREEN_WANTED_KEPT.load(Ordering::Acquire);
    if !screen_kept {
        return SystemDialogAnswer::LetItOpen;
    }
    let from_the_step = INSIDE_THE_TITLE_SYSTEM_DATA_STEP.with(Cell::get);
    let stage = BAND_CALLS
        .get()
        .and_then(|calls| calls.read_i32(SYSTEM_DATA_STAGE_VALUE).ok());
    let Ok(mut dialogs) = BOOT_SCREEN_DIALOGS.lock() else {
        return SystemDialogAnswer::LetItOpen;
    };
    let Some(dialogs) = dialogs.as_mut() else {
        return SystemDialogAnswer::LetItOpen;
    };
    let (answer, line) = dialogs.ask(screen_kept, from_the_step, stage);
    if let Some(line) = line {
        log_line(&line);
    }
    answer
}
