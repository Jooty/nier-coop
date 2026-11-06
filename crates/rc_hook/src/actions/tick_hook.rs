use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicPtr, Ordering};

use rc_game::offsets::SCRIPT_MANAGER_TICK_RVA;

use crate::actions::runner::run_queued_actions;
use crate::build_gate::VerifiedBuild;
use crate::detour::Detour;
use crate::diagnostics::log_line;
use crate::net;

static ORIGINAL_TICK: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();

type ScriptManagerTick = unsafe extern "system" fn(*mut c_void);

pub fn install(build: VerifiedBuild) {
    if HOOKED_BUILD.set(build).is_err() {
        return;
    }
    let replacement = script_manager_tick_detour as *const () as *mut c_void;
    #[expect(
        unsafe_code,
        reason = "hooks the script manager tick; the replacement has the same signature"
    )]
    // SAFETY: SCRIPT_MANAGER_TICK_RVA starts the per-tick function (rcx = script manager, void)
    // in the verified build, and the replacement takes and passes on the same single argument.
    let created = unsafe { Detour::create(&build, SCRIPT_MANAGER_TICK_RVA, replacement) };
    let detour = match created {
        Ok(detour) => detour,
        Err(error) => {
            log_line(&format!("actions: cannot hook the script tick ({error})"));
            return;
        }
    };
    ORIGINAL_TICK.store(detour.trampoline(), Ordering::Release);
    match detour.enable() {
        Ok(()) => log_line("actions: script tick hooked, game-thread queue is live"),
        Err(error) => {
            ORIGINAL_TICK.store(ptr::null_mut(), Ordering::Release);
            log_line(&format!(
                "actions: cannot enable the script tick hook ({error})"
            ));
        }
    }
}

pub fn is_draining() -> bool {
    !ORIGINAL_TICK.load(Ordering::Acquire).is_null()
}

#[expect(
    unsafe_code,
    reason = "calls the original tick through the trampoline MinHook built"
)]
extern "system" fn script_manager_tick_detour(script_manager: *mut c_void) {
    if let Some(build) = HOOKED_BUILD.get()
        && panic::catch_unwind(|| {
            run_queued_actions(build);
            net::publish_local_snapshot(build);
        })
        .is_err()
    {
        log_line("actions: panic while running queued actions");
    }
    let original = ORIGINAL_TICK.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original tick, which has this exact signature.
    unsafe {
        core::mem::transmute::<*mut c_void, ScriptManagerTick>(original)(script_manager);
    }
}
