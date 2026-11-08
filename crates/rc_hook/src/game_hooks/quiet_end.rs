use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::atomic::{AtomicPtr, Ordering};

use rc_game::offsets::{
    PAUSE_OFF_NATIVE_RVA, PAUSE_ON_NATIVE_RVA, START_TALK_NATIVE_RVA,
    START_TALK_UNIQUE_ACTOR_NATIVE_RVA,
};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::hook_slot::hook_address;

static ORIGINAL_START_TALK: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_START_TALK_UNIQUE_ACTOR: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_PAUSE_ON: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_PAUSE_OFF: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

type LuaNative = unsafe extern "system" fn(*mut c_void) -> i32;

pub(super) fn install(build: VerifiedBuild) {
    let base = build.module_base();
    #[expect(
        unsafe_code,
        reason = "hooks the talk start and pause natives by address"
    )]
    // SAFETY: each RVA starts a talk start Lua native (lua args -> result count) in the verified
    // build, and each replacement has the same signature.
    unsafe {
        hook_address(
            "_StartTalk native",
            (base + u64::from(START_TALK_NATIVE_RVA)) as usize,
            start_talk_detour as *const () as *mut c_void,
            &ORIGINAL_START_TALK,
        );
        hook_address(
            "_StartTalkUniqueActorNotChangeEndTalkMotion native",
            (base + u64::from(START_TALK_UNIQUE_ACTOR_NATIVE_RVA)) as usize,
            start_talk_unique_actor_detour as *const () as *mut c_void,
            &ORIGINAL_START_TALK_UNIQUE_ACTOR,
        );
        hook_address(
            "_PauseOn native",
            (base + u64::from(PAUSE_ON_NATIVE_RVA)) as usize,
            pause_on_detour as *const () as *mut c_void,
            &ORIGINAL_PAUSE_ON,
        );
        hook_address(
            "_PauseOff native",
            (base + u64::from(PAUSE_OFF_NATIVE_RVA)) as usize,
            pause_off_detour as *const () as *mut c_void,
            &ORIGINAL_PAUSE_OFF,
        );
    }
}

extern "system" fn start_talk_detour(args: *mut c_void) -> i32 {
    run_unless_quiet(&ORIGINAL_START_TALK, START_TALK_NATIVE_RVA, args)
}

extern "system" fn start_talk_unique_actor_detour(args: *mut c_void) -> i32 {
    run_unless_quiet(
        &ORIGINAL_START_TALK_UNIQUE_ACTOR,
        START_TALK_UNIQUE_ACTOR_NATIVE_RVA,
        args,
    )
}

extern "system" fn pause_on_detour(args: *mut c_void) -> i32 {
    run_unless_quiet(&ORIGINAL_PAUSE_ON, PAUSE_ON_NATIVE_RVA, args)
}

extern "system" fn pause_off_detour(args: *mut c_void) -> i32 {
    run_unless_quiet(&ORIGINAL_PAUSE_OFF, PAUSE_OFF_NATIVE_RVA, args)
}

#[expect(unsafe_code, reason = "calls the original quiet-listed native")]
fn run_unless_quiet(original: &AtomicPtr<c_void>, native_rva: u32, args: *mut c_void) -> i32 {
    let original = original.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    let skipped = panic::catch_unwind(|| crate::pickups::skipped_in_partner_end(native_rva))
        .ok()
        .flatten();
    if let Some((name, callback)) = skipped {
        log_line(&format!(
            "callback follow: skipped {name} inside the partner's {callback}"
        ));
        return 0;
    }
    // SAFETY: the trampoline continues into the original native with this exact signature.
    unsafe { core::mem::transmute::<*mut c_void, LuaNative>(original)(args) }
}
