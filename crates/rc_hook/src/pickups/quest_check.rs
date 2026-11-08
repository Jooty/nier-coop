use core::ffi::c_void;
use core::ptr;
use std::sync::Mutex;
use std::sync::PoisonError;
use std::sync::atomic::{AtomicPtr, AtomicU32, Ordering};

use rc_game::offsets::{
    GET_ITEM_NUM_NATIVE_RVA, QUEST_MANAGER_ADVANCE_STEP_FUNCTION_RVA,
    TRIGGER_MANAGER_UPDATE_FUNCTION_RVA,
};
use rc_game::pickups::{is_shared_item, quest_check_count};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::hook_address;
use crate::pickups::shared_items::{frozen_count_for_poll, partner_count, partner_shares_phase};

static ORIGINAL_GET_ITEM_NUM: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_ADVANCE_STEP: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_TRIGGER_UPDATE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static QUEST_CHECK_DEPTH: AtomicU32 = AtomicU32::new(0);
static TRIGGER_UPDATE_DEPTH: AtomicU32 = AtomicU32::new(0);
static LAST_LOGGED_CHECK: Mutex<(u16, u8, u8)> = Mutex::new((u16::MAX, 0, 0));

type LuaNative = unsafe extern "system" fn(*mut c_void);
type AdvanceStep = unsafe extern "system" fn(*mut c_void, i32) -> u64;
type TriggerUpdate = unsafe extern "system" fn(*mut c_void);

pub fn install(build: VerifiedBuild) {
    let base = build.module_base();
    let hooks: [(&str, u32, *mut c_void, &AtomicPtr<c_void>); 3] = [
        (
            "_GetItemNum native",
            GET_ITEM_NUM_NATIVE_RVA,
            get_item_num_detour as *const () as *mut c_void,
            &ORIGINAL_GET_ITEM_NUM,
        ),
        (
            "QuestManager::AdvanceStep",
            QUEST_MANAGER_ADVANCE_STEP_FUNCTION_RVA,
            advance_step_detour as *const () as *mut c_void,
            &ORIGINAL_ADVANCE_STEP,
        ),
        (
            "TriggerManager::Update",
            TRIGGER_MANAGER_UPDATE_FUNCTION_RVA,
            trigger_update_detour as *const () as *mut c_void,
            &ORIGINAL_TRIGGER_UPDATE,
        ),
    ];
    for (label, rva, replacement, original) in hooks {
        #[expect(unsafe_code, reason = "hooks a quest check entry point by its address")]
        // SAFETY: each RVA starts the named function in the verified build (see docs
        // off-hand/progress.md and story-events.md) and each replacement has its signature.
        unsafe {
            hook_address(
                label,
                (base + u64::from(rva)) as usize,
                replacement,
                original,
            )
        };
    }
}

pub fn count_for_quest(item_id: i32, local: u8) -> u8 {
    if QUEST_CHECK_DEPTH.load(Ordering::Acquire) == 0 || !partner_shares_phase() {
        return local;
    }
    let Ok(item_id) = u16::try_from(item_id) else {
        return local;
    };
    if !is_shared_item(item_id) {
        return local;
    }
    if TRIGGER_UPDATE_DEPTH.load(Ordering::Acquire) > 0
        && let Some(frozen) = frozen_count_for_poll(item_id)
    {
        return frozen;
    }
    let partner = partner_count(item_id);
    if partner == 0 {
        return local;
    }
    note_quest_check(item_id, local, partner);
    quest_check_count(local, partner)
}

fn note_quest_check(item_id: u16, local: u8, partner: u8) {
    let mut last = LAST_LOGGED_CHECK
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if *last == (item_id, local, partner) {
        return;
    }
    *last = (item_id, local, partner);
    log_line(&format!(
        "shared items: quest check for item {item_id}: you {local} + partner {partner}"
    ));
}

extern "system" fn get_item_num_detour(args: *mut c_void) {
    let original = ORIGINAL_GET_ITEM_NUM.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    QUEST_CHECK_DEPTH.fetch_add(1, Ordering::AcqRel);
    #[expect(unsafe_code, reason = "calls the original _GetItemNum native")]
    // SAFETY: the trampoline continues into the original native, which has this signature.
    unsafe {
        core::mem::transmute::<*mut c_void, LuaNative>(original)(args)
    };
    QUEST_CHECK_DEPTH.fetch_sub(1, Ordering::AcqRel);
}

extern "system" fn advance_step_detour(manager: *mut c_void, quest: i32) -> u64 {
    let original = ORIGINAL_ADVANCE_STEP.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    QUEST_CHECK_DEPTH.fetch_add(1, Ordering::AcqRel);
    #[expect(unsafe_code, reason = "calls the original QuestManager::AdvanceStep")]
    // SAFETY: the trampoline continues into the original function with this signature.
    let result =
        unsafe { core::mem::transmute::<*mut c_void, AdvanceStep>(original)(manager, quest) };
    QUEST_CHECK_DEPTH.fetch_sub(1, Ordering::AcqRel);
    result
}

extern "system" fn trigger_update_detour(manager: *mut c_void) {
    let original = ORIGINAL_TRIGGER_UPDATE.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    QUEST_CHECK_DEPTH.fetch_add(1, Ordering::AcqRel);
    TRIGGER_UPDATE_DEPTH.fetch_add(1, Ordering::AcqRel);
    #[expect(unsafe_code, reason = "calls the original TriggerManager::Update")]
    // SAFETY: the trampoline continues into the original function with this signature.
    unsafe {
        core::mem::transmute::<*mut c_void, TriggerUpdate>(original)(manager)
    };
    TRIGGER_UPDATE_DEPTH.fetch_sub(1, Ordering::AcqRel);
    QUEST_CHECK_DEPTH.fetch_sub(1, Ordering::AcqRel);
}
