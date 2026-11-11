use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::atomic::{AtomicPtr, AtomicU64, Ordering};

use replicant_game::coop_pause::{
    UI_DIALOG_MASK_SAVED_AT_OPEN_OFFSET, popup_saved_mask_after_slot_zero_write,
};
use replicant_game::dialogue::CONTROL_MASK_SET_SLOT_ZERO_RVA;
use replicant_game::marker::UI_DIALOG_STATE_RVA;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::menu_no_pause::partner_connected;
use nc_hook::detour::hook_address;

static ORIGINAL_SET_SLOT_ZERO: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static MODULE_BASE: AtomicU64 = AtomicU64::new(0);

type SetMaskSlot = unsafe extern "system" fn(*mut c_void, u32);

pub(super) fn install(build: VerifiedBuild) {
    let base = build.module_base();
    MODULE_BASE.store(base, Ordering::Relaxed);
    #[expect(
        unsafe_code,
        reason = "hooks GameManager::SetControlMaskSlot0 by address"
    )]
    // SAFETY: the RVA starts `void SetControlMaskSlot0(void* gameManager, uint mask)` in the
    // verified build, and the replacement has the same signature.
    unsafe {
        hook_address(
            "GameManager::SetControlMaskSlot0",
            (base + u64::from(CONTROL_MASK_SET_SLOT_ZERO_RVA)) as usize,
            set_slot_zero_detour as *const () as *mut c_void,
            &ORIGINAL_SET_SLOT_ZERO,
        );
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original control mask slot 0 setter through the trampoline MinHook built"
)]
extern "system" fn set_slot_zero_detour(game_state: *mut c_void, mask: u32) {
    let original = ORIGINAL_SET_SLOT_ZERO.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    if partner_connected() {
        let _ = panic::catch_unwind(|| follow_write(mask));
    }
    // SAFETY: the trampoline continues into the original setter, which has this signature.
    unsafe { core::mem::transmute::<*mut c_void, SetMaskSlot>(original)(game_state, mask) }
}

#[expect(
    unsafe_code,
    reason = "reads the UI dialog state and writes its saved mask in the game's image"
)]
fn follow_write(written: u32) {
    let base = MODULE_BASE.load(Ordering::Relaxed);
    if base == 0 {
        return;
    }
    let state_address = (base + u64::from(UI_DIALOG_STATE_RVA)) as *const i32;
    // SAFETY: the UI dialog object is fixed, aligned data in the verified build's image and its
    // saved-mask dword sits inside it; plain volatile accesses on the game thread cannot fault.
    unsafe {
        let state = ptr::read_volatile(state_address);
        let saved_address = (base
            + u64::from(UI_DIALOG_STATE_RVA)
            + UI_DIALOG_MASK_SAVED_AT_OPEN_OFFSET) as *mut u32;
        let saved = ptr::read_volatile(saved_address);
        let followed = popup_saved_mask_after_slot_zero_write(state, saved, written);
        if followed == saved {
            return;
        }
        ptr::write_volatile(saved_address, followed);
        log_line(&format!(
            "control: the pop-up's saved mask follows the game, {saved:#x} -> {followed:#x} (slot 0 written {written:#x})"
        ));
    }
}
