use core::ffi::c_void;
use core::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

use replicant_game::GameMemory;
use replicant_game::dialogue::{
    CONTROL_MASK_SET_SLOT_ONE_RVA, CONTROL_MASK_TALK_SLOT_RVA, GAME_STATE_RVA,
};

use crate::build_gate::VerifiedBuild;
use crate::dialogue::tags::mirror_action_running;
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;

static ORIGINAL_SET_SLOT_ONE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static OTHER_CODE_WROTE_SLOT_ONE: AtomicBool = AtomicBool::new(false);
static MIRROR_WROTE_SLOT_ONE: AtomicBool = AtomicBool::new(false);

type SetMaskSlot = unsafe extern "system" fn(*mut c_void, u32);

pub(super) fn install(build: VerifiedBuild) {
    let base = build.module_base();
    #[expect(
        unsafe_code,
        reason = "hooks GameManager::SetControlMaskSlot1 by address"
    )]
    // SAFETY: the RVA starts `void SetControlMaskSlot1(void* gameManager, uint mask)` in the
    // verified build (see docs off-hand/control-state.md), and the replacement has the same signature.
    unsafe {
        hook_address(
            "GameManager::SetControlMaskSlot1",
            (base + u64::from(CONTROL_MASK_SET_SLOT_ONE_RVA)) as usize,
            set_slot_one_detour as *const () as *mut c_void,
            &ORIGINAL_SET_SLOT_ONE,
        );
    }
}

pub(super) fn take_other_code_wrote_slot_one() -> bool {
    OTHER_CODE_WROTE_SLOT_ONE.swap(false, Ordering::AcqRel)
}

pub(super) fn slot_one_written_by_mirror() -> bool {
    MIRROR_WROTE_SLOT_ONE.load(Ordering::Acquire)
}

pub(super) fn forget_mirror_wrote_slot_one() {
    MIRROR_WROTE_SLOT_ONE.store(false, Ordering::Release);
}

pub(super) fn read_slot_one(build: &VerifiedBuild) -> u32 {
    let memory = InProcessMemory::new(build);
    memory
        .read_i32(memory.module_base() + u64::from(CONTROL_MASK_TALK_SLOT_RVA))
        .map_or(0, i32::cast_unsigned)
}

#[expect(
    unsafe_code,
    reason = "calls the original control mask slot 1 setter through the trampoline MinHook built"
)]
pub(super) fn write_slot_one(build: &VerifiedBuild, mask: u32) -> bool {
    let original = ORIGINAL_SET_SLOT_ONE.load(Ordering::Acquire);
    if original.is_null() {
        return false;
    }
    // SAFETY: the trampoline continues into the original setter (game state, mask), called on the
    // game thread like `_StartTalk` does; going around the detour keeps the hold's own write
    // from looking like another system's.
    unsafe {
        core::mem::transmute::<*mut c_void, SetMaskSlot>(original)(
            (build.module_base() + u64::from(GAME_STATE_RVA)) as *mut c_void,
            mask,
        );
    }
    true
}

#[expect(
    unsafe_code,
    reason = "calls the original control mask slot 1 setter through the trampoline MinHook built"
)]
extern "system" fn set_slot_one_detour(game_state: *mut c_void, mask: u32) {
    // The mirror's own window calls set the mask too; that write must not look like an own talk.
    if mirror_action_running() {
        MIRROR_WROTE_SLOT_ONE.store(true, Ordering::Release);
    } else {
        MIRROR_WROTE_SLOT_ONE.store(false, Ordering::Release);
        OTHER_CODE_WROTE_SLOT_ONE.store(true, Ordering::Release);
    }
    let original = ORIGINAL_SET_SLOT_ONE.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original setter, which has this signature.
    unsafe { core::mem::transmute::<*mut c_void, SetMaskSlot>(original)(game_state, mask) }
}
