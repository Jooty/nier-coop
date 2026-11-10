use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicPtr, Ordering};

use crate::build_gate::VerifiedBuild;
use crate::detour::Detour;
use crate::diagnostics::log_line;

static ORIGINAL_TICK: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
static ON_TICK: OnceLock<fn(&VerifiedBuild)> = OnceLock::new();

type GameThreadTick = unsafe extern "system" fn(*mut c_void);
type GameThreadTickWithResult = unsafe extern "system" fn(*mut c_void) -> usize;

pub fn install_game_thread_tick(build: VerifiedBuild, tick_rva: u32, on_tick: fn(&VerifiedBuild)) {
    install(
        build,
        tick_rva,
        on_tick,
        game_thread_tick_detour as *const () as *mut c_void,
    );
}

/// For a per-tick function whose caller tests its result: the detour hands the result back.
pub fn install_game_thread_tick_with_result(
    build: VerifiedBuild,
    tick_rva: u32,
    on_tick: fn(&VerifiedBuild),
) {
    install(
        build,
        tick_rva,
        on_tick,
        game_thread_tick_with_result_detour as *const () as *mut c_void,
    );
}

fn install(
    build: VerifiedBuild,
    tick_rva: u32,
    on_tick: fn(&VerifiedBuild),
    replacement: *mut c_void,
) {
    if HOOKED_BUILD.set(build).is_err() {
        return;
    }
    let _ = ON_TICK.set(on_tick);
    #[expect(
        unsafe_code,
        reason = "hooks the per-tick function; the replacement has the same signature"
    )]
    // SAFETY: `tick_rva` starts a per-tick function (one pointer argument) in the verified build,
    // and each caller above passes the replacement that matches its result: none, or one register.
    let created = unsafe { Detour::create(&build, tick_rva, replacement) };
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
extern "system" fn game_thread_tick_detour(context: *mut c_void) {
    run_on_tick();
    let original = ORIGINAL_TICK.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original tick, which has this exact signature.
    unsafe {
        core::mem::transmute::<*mut c_void, GameThreadTick>(original)(context);
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original tick through the trampoline MinHook built"
)]
extern "system" fn game_thread_tick_with_result_detour(context: *mut c_void) -> usize {
    run_on_tick();
    let original = ORIGINAL_TICK.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    // SAFETY: the trampoline continues into the original tick, which takes this one argument and
    // returns its result in one register.
    unsafe { core::mem::transmute::<*mut c_void, GameThreadTickWithResult>(original)(context) }
}

fn run_on_tick() {
    if let (Some(build), Some(on_tick)) = (HOOKED_BUILD.get(), ON_TICK.get())
        && panic::catch_unwind(|| on_tick(build)).is_err()
    {
        log_line("actions: panic while running queued actions");
    }
}
