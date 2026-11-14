use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::{Mutex, PoisonError};

use replicant_game::world_objects::{CUBE_BREAK_FUNCTION_RVA, MAX_PENDING_PARTNER_BREAKS};

use crate::build_gate::VerifiedBuild;
use crate::game_hooks::hook_address;

static ORIGINAL_BREAK: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static COPYING_PARTNER_BREAK: AtomicBool = AtomicBool::new(false);
static LOCAL_BREAKS: Mutex<Vec<u64>> = Mutex::new(Vec::new());

type CubeBreak = unsafe extern "system" fn(*mut c_void);

pub fn install(build: VerifiedBuild) {
    #[expect(
        unsafe_code,
        reason = "hooks the attack box cube's break method by address"
    )]
    // SAFETY: the RVA starts CActorAttackBox's break method (rcx actor, nothing else read: it
    // sets its own edx, r8d and r9d) in the verified build; the replacement has the same signature.
    unsafe {
        hook_address(
            "CActorAttackBox::Break",
            (build.module_base() + u64::from(CUBE_BREAK_FUNCTION_RVA)) as usize,
            break_detour as *const () as *mut c_void,
            &ORIGINAL_BREAK,
        );
    }
}

/// Breaks this game's copy of a cube through the cube's own break method, as the partner's break.
pub(super) fn break_here(actor: u64) {
    let original = ORIGINAL_BREAK.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    COPYING_PARTNER_BREAK.store(true, Ordering::Release);
    #[expect(
        unsafe_code,
        reason = "calls the cube's original break method through the trampoline"
    )]
    // SAFETY: the caller checked the actor is a live cube of this map, the trampoline continues
    // into the original method with this exact signature, and this runs on the game thread.
    unsafe {
        core::mem::transmute::<*mut c_void, CubeBreak>(original)(actor as *mut c_void);
    }
    COPYING_PARTNER_BREAK.store(false, Ordering::Release);
}

pub(super) fn take_local_breaks() -> Vec<u64> {
    let mut breaks = LOCAL_BREAKS.lock().unwrap_or_else(PoisonError::into_inner);
    core::mem::take(&mut *breaks)
}

#[expect(
    unsafe_code,
    reason = "calls the original break method through the trampoline MinHook built"
)]
extern "system" fn break_detour(actor: *mut c_void) {
    let original = ORIGINAL_BREAK.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    if !COPYING_PARTNER_BREAK.load(Ordering::Acquire) {
        let _ = panic::catch_unwind(|| note_local_break(actor as u64));
    }
    // SAFETY: the trampoline continues into the original method, which has this exact signature.
    unsafe { core::mem::transmute::<*mut c_void, CubeBreak>(original)(actor) }
}

fn note_local_break(actor: u64) {
    let Ok(mut breaks) = LOCAL_BREAKS.try_lock() else {
        return;
    };
    if breaks.len() < MAX_PENDING_PARTNER_BREAKS && !breaks.contains(&actor) {
        breaks.push(actor);
    }
}
