use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{Mutex, PoisonError};

use replicant_game::offsets::{RESTART_PHASE_NATIVE_RVA, RESTORE_SAVE_DATA_NATIVE_RVA};

use crate::build_gate::VerifiedBuild;
use crate::game_hooks::{hook_address, note_native};

static ORIGINAL_RESTART_PHASE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_RESTORE_SAVE_DATA: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static SIGNALS: Mutex<(u32, String)> = Mutex::new((0, String::new()));

type Native = unsafe extern "system" fn(*mut c_void);

pub(super) fn install(build: VerifiedBuild) {
    let base = build.module_base();
    #[expect(
        unsafe_code,
        reason = "hooks the restart-phase and restore-save-data script natives by address"
    )]
    // SAFETY: the RVAs start _RestartPhase and _RestoreSaveData (args block) in the verified build
    // (docs symbols.tsv, binding table 0x1306710) and the detours have the same signature.
    unsafe {
        hook_address(
            "LuaNative::RestartPhase",
            (base + u64::from(RESTART_PHASE_NATIVE_RVA)) as usize,
            restart_phase_detour as *const () as *mut c_void,
            &ORIGINAL_RESTART_PHASE,
        );
        hook_address(
            "LuaNative::RestoreSaveData",
            (base + u64::from(RESTORE_SAVE_DATA_NATIVE_RVA)) as usize,
            restore_save_data_detour as *const () as *mut c_void,
            &ORIGINAL_RESTORE_SAVE_DATA,
        );
    }
}

pub(super) fn note_restart(why: &str) {
    let mut signals = SIGNALS.lock().unwrap_or_else(PoisonError::into_inner);
    signals.0 = signals.0.wrapping_add(1);
    signals.1 = why.to_owned();
}

pub(crate) fn restart_signals() -> (u32, String) {
    SIGNALS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

extern "system" fn restart_phase_detour(args: *mut c_void) {
    let _ = panic::catch_unwind(|| {
        note_native("_RestartPhase");
        note_restart("_RestartPhase native");
    });
    call_original(&ORIGINAL_RESTART_PHASE, args);
}

extern "system" fn restore_save_data_detour(args: *mut c_void) {
    let _ = panic::catch_unwind(|| {
        note_native("_RestoreSaveData");
        note_restart("_RestoreSaveData native");
    });
    call_original(&ORIGINAL_RESTORE_SAVE_DATA, args);
}

#[expect(
    unsafe_code,
    reason = "calls the original native through the trampoline"
)]
fn call_original(original: &AtomicPtr<c_void>, args: *mut c_void) {
    let trampoline = original.load(Ordering::Acquire);
    if !trampoline.is_null() {
        // SAFETY: the trampoline continues into the original native, which has this signature.
        unsafe { core::mem::transmute::<*mut c_void, Native>(trampoline)(args) };
    }
}
