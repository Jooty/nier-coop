use core::ffi::c_void;
use core::ptr;
use std::panic::{self, AssertUnwindSafe};
use std::sync::Mutex;
use std::sync::PoisonError;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicPtr, Ordering};

use rc_game::Replicant;
use rc_game::new_game::{MENU_RESULT_NAME_CONFIRMED, MENU_RESULT_NEW_GAME, TitleStep, title_step};
use rc_game::player_name::{NAME_FIELD_LEN, name_field_bytes};

use crate::boot::first_save;
use crate::boot::game_access::{
    NATIVE_FINISHED, NATIVE_STILL_RUNNING, START_INPUT_PLAYER_NAME_RVA, START_TITLE_RVA, build,
    call_original, game_memory, hook_native, remember_build, system_data_loaded, write_menu_result,
};
use crate::boot::title_hold::{hold_for_partner, start_hold_clock};
use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;

const NO_SLOT: i32 = -1;

static ORIGINAL_START_TITLE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_START_INPUT_PLAYER_NAME: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static TITLE_ANSWER_PENDING: AtomicBool = AtomicBool::new(false);
static NAME_ANSWER_PENDING: AtomicBool = AtomicBool::new(false);
static NEW_GAME_SLOT: AtomicI32 = AtomicI32::new(NO_SLOT);
static FIRST_SAVE_ALLOWED: AtomicBool = AtomicBool::new(false);
static NAME_FIELD: Mutex<Option<[u8; NAME_FIELD_LEN]>> = Mutex::new(None);

pub fn install(
    build: VerifiedBuild,
    slot: u8,
    name: Option<&str>,
    linked: bool,
    first_save_allowed: bool,
) {
    if !remember_build(build) {
        return;
    }
    start_hold_clock(linked);
    NEW_GAME_SLOT.store(i32::from(slot), Ordering::Release);
    FIRST_SAVE_ALLOWED.store(first_save_allowed, Ordering::Release);
    TITLE_ANSWER_PENDING.store(true, Ordering::Release);
    hook_native(
        "_StartTitle (new game)",
        START_TITLE_RVA,
        start_title_detour as *const () as *mut c_void,
        &ORIGINAL_START_TITLE,
    );
    match name.and_then(name_field_bytes) {
        Some(field) => {
            *NAME_FIELD.lock().unwrap_or_else(PoisonError::into_inner) = Some(field);
            hook_native(
                "_StartInputPlayerName",
                START_INPUT_PLAYER_NAME_RVA,
                start_input_player_name_detour as *const () as *mut c_void,
                &ORIGINAL_START_INPUT_PLAYER_NAME,
            );
        }
        None => log_line("boot: new game has no usable name, the name box stays"),
    }
    log_line(&format!(
        "boot: starting a new game in slot {slot} (first save {})",
        if first_save_allowed { "on" } else { "off" }
    ));
}

extern "system" fn start_title_detour(argument: *mut c_void) -> i32 {
    panic::catch_unwind(AssertUnwindSafe(|| answer_title(argument))).unwrap_or_else(|_| {
        log_line("boot: the title detour panicked, running the original");
        call_original(&ORIGINAL_START_TITLE, argument)
    })
}

extern "system" fn start_input_player_name_detour(argument: *mut c_void) -> i32 {
    panic::catch_unwind(AssertUnwindSafe(|| answer_name_box(argument))).unwrap_or_else(|_| {
        log_line("boot: the name box detour panicked, running the original");
        call_original(&ORIGINAL_START_INPUT_PLAYER_NAME, argument)
    })
}

fn answer_title(argument: *mut c_void) -> i32 {
    if !TITLE_ANSWER_PENDING.load(Ordering::Acquire) {
        return call_original(&ORIGINAL_START_TITLE, argument);
    }
    if hold_for_partner() {
        return NATIVE_STILL_RUNNING;
    }
    TITLE_ANSWER_PENDING.store(false, Ordering::Release);
    NAME_ANSWER_PENDING.store(true, Ordering::Release);
    if title_step(system_data_loaded()) == TitleStep::ShowRealTitle {
        log_line("boot: no system data yet, showing the title once so the game creates it");
        return call_original(&ORIGINAL_START_TITLE, argument);
    }
    if let Err(error) = write_menu_result(MENU_RESULT_NEW_GAME) {
        log_line(&format!("boot: cannot skip the title ({error})"));
        return call_original(&ORIGINAL_START_TITLE, argument);
    }
    log_line("boot: title skipped, new game");
    arm_first_save(NEW_GAME_SLOT.load(Ordering::Acquire));
    NATIVE_FINISHED
}

fn answer_name_box(argument: *mut c_void) -> i32 {
    if !NAME_ANSWER_PENDING.swap(false, Ordering::AcqRel) {
        return call_original(&ORIGINAL_START_INPUT_PLAYER_NAME, argument);
    }
    let field = *NAME_FIELD.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(field) = field else {
        return call_original(&ORIGINAL_START_INPUT_PLAYER_NAME, argument);
    };
    if let Err(error) = write_name_field(&field) {
        log_line(&format!("boot: cannot set the new game's name ({error})"));
        return call_original(&ORIGINAL_START_INPUT_PLAYER_NAME, argument);
    }
    let name_bytes = field.iter().take_while(|byte| **byte != 0).count();
    if let Err(error) = write_menu_result(MENU_RESULT_NAME_CONFIRMED) {
        log_line(&format!("boot: cannot confirm the name ({error})"));
        return call_original(&ORIGINAL_START_INPUT_PLAYER_NAME, argument);
    }
    log_line(&format!("boot: new game, name set ({name_bytes} bytes)"));
    arm_first_save(NEW_GAME_SLOT.load(Ordering::Acquire));
    NATIVE_FINISHED
}

fn write_name_field(field: &[u8; NAME_FIELD_LEN]) -> Result<(), String> {
    let build = build().ok_or("no build")?;
    let mut game = Replicant::new(InProcessMemory::new(build));
    game.set_save_name_field(field)
        .map_err(|error| error.to_string())
}

fn arm_first_save(slot: i32) {
    if !FIRST_SAVE_ALLOWED.load(Ordering::Acquire) {
        log_line("boot: first save off (no co-op file redirect without a link)");
        return;
    }
    if game_memory().is_none() {
        return;
    }
    if let Ok(slot) = u8::try_from(slot) {
        first_save::arm(slot);
    }
}
