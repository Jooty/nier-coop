use core::ffi::c_void;
use core::ptr;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicPtr, Ordering};

use replicant_game::offsets::{
    COPY_SLOT_TO_BLOCK_RVA, SAVE_FILE_IMAGE_RVA, START_LOAD_RVA, START_TITLE_RVA,
};

use crate::boot::coop_reload::{ReloadStep, reload_step};
use crate::boot::game_access::{
    NATIVE_FINISHED, NATIVE_STILL_RUNNING, build, call_original, hook_native, remember_build,
    slot_holds_a_save, write_menu_result,
};
use crate::boot::title_hold::{hold_for_partner, start_hold_clock};
use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;

const RESULT_CANCELLED: i32 = 2;
const RESULT_LOAD_GAME: i32 = 5;
const RESULT_DONE: i32 = 6;
const NO_BOOT_SLOT: i32 = -1;

static ORIGINAL_START_TITLE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_START_LOAD: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static TITLE_SKIP_PENDING: AtomicBool = AtomicBool::new(false);
static SLOT_COPY_PENDING: AtomicBool = AtomicBool::new(false);
static BOOT_SLOT: AtomicI32 = AtomicI32::new(NO_BOOT_SLOT);

type CopySlotToBlockFn = unsafe extern "system" fn(*mut c_void, i32) -> u8;

pub fn install(build: VerifiedBuild, slot: u8, linked: bool) {
    if !remember_build(build) {
        return;
    }
    start_hold_clock(linked);
    BOOT_SLOT.store(i32::from(slot), Ordering::Release);
    // _StartLoad first: a title skipped before its hook is live lets the original open the load menu.
    hook_native(
        "_StartLoad",
        START_LOAD_RVA,
        start_load_detour as *const () as *mut c_void,
        &ORIGINAL_START_LOAD,
    );
    TITLE_SKIP_PENDING.store(true, Ordering::Release);
    hook_native(
        "_StartTitle",
        START_TITLE_RVA,
        start_title_detour as *const () as *mut c_void,
        &ORIGINAL_START_TITLE,
    );
    log_line(&format!("boot: going straight to save slot {slot}"));
}

#[expect(unsafe_code, reason = "calls SaveData::CopySlotToBlock by its address")]
fn copy_slot_to_block(slot: i32) -> bool {
    let Some(build) = build() else {
        return false;
    };
    let function = (build.module_base() + COPY_SLOT_TO_BLOCK_RVA) as usize;
    let image = (build.module_base() + SAVE_FILE_IMAGE_RVA) as usize as *mut c_void;
    // SAFETY: the RVA starts SaveData::CopySlotToBlock (image, slot) -> bool in the verified build.
    let copied = unsafe { core::mem::transmute::<usize, CopySlotToBlockFn>(function)(image, slot) };
    copied != 0
}

extern "system" fn start_title_detour(argument: *mut c_void) -> i32 {
    panic::catch_unwind(AssertUnwindSafe(|| answer_title(argument))).unwrap_or_else(|_| {
        TITLE_SKIP_PENDING.store(false, Ordering::Release);
        log_line("boot: the title detour panicked, running the original");
        call_original(&ORIGINAL_START_TITLE, argument)
    })
}

fn answer_title(argument: *mut c_void) -> i32 {
    if !TITLE_SKIP_PENDING.load(Ordering::Acquire) {
        return call_original(&ORIGINAL_START_TITLE, argument);
    }
    if hold_for_partner() {
        return NATIVE_STILL_RUNNING;
    }
    match reload_step() {
        ReloadStep::Waiting => return NATIVE_STILL_RUNNING,
        ReloadStep::Failed => {
            TITLE_SKIP_PENDING.store(false, Ordering::Release);
            log_line("boot: the co-op file could not be read, showing the title");
            return call_original(&ORIGINAL_START_TITLE, argument);
        }
        ReloadStep::Loaded => {}
    }
    TITLE_SKIP_PENDING.store(false, Ordering::Release);
    let slot = BOOT_SLOT.load(Ordering::Acquire);
    if !slot_holds_a_save(slot) {
        log_line(&format!(
            "boot: slot {slot} holds no save, showing the title"
        ));
        return call_original(&ORIGINAL_START_TITLE, argument);
    }
    if let Err(error) = write_menu_result(RESULT_LOAD_GAME) {
        log_line(&format!("boot: cannot skip the title ({error})"));
        return call_original(&ORIGINAL_START_TITLE, argument);
    }
    SLOT_COPY_PENDING.store(true, Ordering::Release);
    log_line("boot: title skipped");
    NATIVE_FINISHED
}

extern "system" fn start_load_detour(argument: *mut c_void) -> i32 {
    panic::catch_unwind(AssertUnwindSafe(|| answer_load(argument))).unwrap_or_else(|_| {
        log_line("boot: the load detour panicked, running the original");
        call_original(&ORIGINAL_START_LOAD, argument)
    })
}

fn answer_load(argument: *mut c_void) -> i32 {
    if !SLOT_COPY_PENDING.swap(false, Ordering::AcqRel) {
        return call_original(&ORIGINAL_START_LOAD, argument);
    }
    let slot = BOOT_SLOT.load(Ordering::Acquire);
    let loaded = copy_slot_to_block(slot);
    log_line(&format!("boot: slot {slot} copied={loaded}"));
    let result = if loaded {
        RESULT_DONE
    } else {
        RESULT_CANCELLED
    };
    let _ = write_menu_result(result);
    NATIVE_FINISHED
}
