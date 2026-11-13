use core::ffi::c_void;
use core::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, Ordering};

use automata_game::address::REQUEST_SAVE_SLOT_FUNCTION;
use automata_game::build::variant_of_image_size;
use automata_game::start::HIGHEST_SAVE_SLOT;
use nc_hook::build_gate::VerifiedBuild;
use nc_hook::diagnostics::log_line;

use crate::game_calls::GameCalls;
use crate::game_hooks::story_gate::call_is_ours;

const NO_SLOT_SAVED_YET: u32 = u32::MAX;

static ORIGINAL_REQUEST_SAVE_SLOT: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static WATCH_INSTALLED: AtomicBool = AtomicBool::new(false);
static SAVES_ASKED_BY_THE_GAME: AtomicU32 = AtomicU32::new(0);
static LAST_SLOT_SAVED: AtomicU32 = AtomicU32::new(NO_SLOT_SAVED_YET);

type OneArgument = unsafe extern "system" fn(usize) -> usize;

pub fn install_save_request_watch(build: VerifiedBuild, calls: &GameCalls) -> bool {
    if WATCH_INSTALLED.load(Ordering::Acquire) {
        return true;
    }
    let Some(variant) = variant_of_image_size(build.size_of_image()) else {
        return false;
    };
    if !calls.slot_calls_verified() {
        log_line(
            "save together: the save calls of this exe are not verified, the save request is not watched",
        );
        return false;
    }
    let address = build.module_base() as usize + REQUEST_SAVE_SLOT_FUNCTION.rva(variant) as usize;
    #[expect(
        unsafe_code,
        reason = "hooks the game's save slot request; the replacement forwards its one argument"
    )]
    // SAFETY: the address is SaveMgr::RequestSaveSlot of the verified build (it takes the slot);
    // the replacement has that signature and returns the original's rax.
    let hooked = unsafe {
        nc_hook::detour::hook_address(
            "save slot request (save together)",
            address,
            request_save_slot_detour as *const () as *mut c_void,
            &ORIGINAL_REQUEST_SAVE_SLOT,
        )
    };
    WATCH_INSTALLED.store(hooked, Ordering::Release);
    hooked
}

pub fn take_saves_asked_by_the_game() -> u32 {
    SAVES_ASKED_BY_THE_GAME.swap(0, Ordering::AcqRel)
}

pub fn last_slot_saved() -> Option<u32> {
    let slot = LAST_SLOT_SAVED.load(Ordering::Acquire);
    (slot != NO_SLOT_SAVED_YET).then_some(slot)
}

#[expect(
    unsafe_code,
    reason = "calls the original save slot request through the trampoline MinHook built"
)]
extern "system" fn request_save_slot_detour(slot: usize) -> usize {
    let original = ORIGINAL_REQUEST_SAVE_SLOT.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    if slot as u32 <= HIGHEST_SAVE_SLOT {
        LAST_SLOT_SAVED.store(slot as u32, Ordering::Release);
    }
    if !call_is_ours() {
        SAVES_ASKED_BY_THE_GAME.fetch_add(1, Ordering::AcqRel);
    }
    // SAFETY: the trampoline continues into the original function with the argument received.
    unsafe { core::mem::transmute::<*mut c_void, OneArgument>(original)(slot) }
}
