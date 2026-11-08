use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicPtr, Ordering};

use rc_game::offsets::{
    ITEM_MANAGER_GET_COUNT_FUNCTION_RVA, ITEM_MANAGER_REMOVE_FUNCTION_RVA, ITEM_MANAGER_RVA,
    ITEM_MANAGER_SET_COUNT_FUNCTION_RVA,
};

use crate::build_gate::VerifiedBuild;
use crate::game_hooks::hook_address;
use crate::pickups::quest_check::count_for_quest;
use crate::pickups::shared_items::announce_local_count;

static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
static ORIGINAL_GET_COUNT: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_SET_COUNT: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_REMOVE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

type GetCount = unsafe extern "system" fn(*mut c_void, i32) -> u32;
type ChangeCount = unsafe extern "system" fn(*mut c_void, i32, i32) -> u64;

pub fn install(build: VerifiedBuild) {
    if HOOKED_BUILD.set(build).is_err() {
        return;
    }
    let base = build.module_base();
    let hooks: [(&str, u32, *mut c_void, &AtomicPtr<c_void>); 3] = [
        (
            "ItemManager::GetCount",
            ITEM_MANAGER_GET_COUNT_FUNCTION_RVA,
            get_count_detour as *const () as *mut c_void,
            &ORIGINAL_GET_COUNT,
        ),
        (
            "ItemManager::SetCount",
            ITEM_MANAGER_SET_COUNT_FUNCTION_RVA,
            set_count_detour as *const () as *mut c_void,
            &ORIGINAL_SET_COUNT,
        ),
        (
            "ItemManager::Remove",
            ITEM_MANAGER_REMOVE_FUNCTION_RVA,
            remove_detour as *const () as *mut c_void,
            &ORIGINAL_REMOVE,
        ),
    ];
    for (label, rva, replacement, original) in hooks {
        #[expect(unsafe_code, reason = "hooks an ItemManager function by its address")]
        // SAFETY: each RVA starts the named ItemManager function in the verified build and each
        // replacement has that function's signature (manager, item id[, count]).
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

pub fn hooked_build() -> Option<&'static VerifiedBuild> {
    HOOKED_BUILD.get()
}

#[expect(
    unsafe_code,
    reason = "calls the original ItemManager::GetCount through the trampoline MinHook built"
)]
pub fn local_count(item_id: u16) -> Option<u8> {
    let original = ORIGINAL_GET_COUNT.load(Ordering::Acquire);
    let build = HOOKED_BUILD.get()?;
    if original.is_null() {
        return None;
    }
    let manager = (build.module_base() + u64::from(ITEM_MANAGER_RVA)) as usize as *mut c_void;
    // SAFETY: the trampoline continues into the original GetCount (manager, id -> count byte);
    // the manager is the game's global item manager; this runs on the game thread.
    let count = unsafe {
        core::mem::transmute::<*mut c_void, GetCount>(original)(manager, i32::from(item_id))
    };
    Some((count & 0xFF) as u8)
}

#[expect(
    unsafe_code,
    reason = "calls the original ItemManager::Remove through the trampoline MinHook built"
)]
pub fn remove_for_partner(item_id: u16, count: u8) -> Option<u64> {
    let original = ORIGINAL_REMOVE.load(Ordering::Acquire);
    let build = HOOKED_BUILD.get()?;
    if original.is_null() {
        return None;
    }
    let manager = (build.module_base() + u64::from(ITEM_MANAGER_RVA)) as usize as *mut c_void;
    // SAFETY: the trampoline continues into the original Remove (manager, id, count); the
    // manager is the game's global item manager; this runs on the game thread.
    let removed = unsafe {
        core::mem::transmute::<*mut c_void, ChangeCount>(original)(
            manager,
            i32::from(item_id),
            i32::from(count),
        )
    };
    announce_after_change(i32::from(item_id));
    Some(removed)
}

#[expect(
    unsafe_code,
    reason = "calls the original ItemManager::GetCount through the trampoline MinHook built"
)]
extern "system" fn get_count_detour(manager: *mut c_void, item_id: i32) -> u32 {
    let original = ORIGINAL_GET_COUNT.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    // SAFETY: the trampoline continues into the original GetCount with this exact signature.
    let local =
        (unsafe { core::mem::transmute::<*mut c_void, GetCount>(original)(manager, item_id) }
            & 0xFF) as u8;
    let from_menu_quest_page =
        panic::catch_unwind(|| crate::pickups::menu_read::note_count_read(item_id))
            .unwrap_or(false);
    let counted = panic::catch_unwind(|| count_for_quest(item_id, local, from_menu_quest_page))
        .unwrap_or(local);
    u32::from(counted)
}

#[expect(
    unsafe_code,
    reason = "calls the original ItemManager::SetCount through the trampoline MinHook built"
)]
extern "system" fn set_count_detour(manager: *mut c_void, item_id: i32, count: i32) -> u64 {
    let original = ORIGINAL_SET_COUNT.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    // SAFETY: the trampoline continues into the original SetCount with this exact signature.
    let result = unsafe {
        core::mem::transmute::<*mut c_void, ChangeCount>(original)(manager, item_id, count)
    };
    announce_after_change(item_id);
    result
}

#[expect(
    unsafe_code,
    reason = "calls the original ItemManager::Remove through the trampoline MinHook built"
)]
extern "system" fn remove_detour(manager: *mut c_void, item_id: i32, count: i32) -> u64 {
    let original = ORIGINAL_REMOVE.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    // SAFETY: the trampoline continues into the original Remove with this exact signature.
    let result = unsafe {
        core::mem::transmute::<*mut c_void, ChangeCount>(original)(manager, item_id, count)
    };
    announce_after_change(item_id);
    result
}

fn announce_after_change(item_id: i32) {
    let Ok(item_id) = u16::try_from(item_id) else {
        return;
    };
    let _ = panic::catch_unwind(|| {
        if let Some(count) = local_count(item_id) {
            announce_local_count(item_id, count);
        }
    });
}
