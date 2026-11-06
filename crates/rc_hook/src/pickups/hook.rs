use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicPtr, Ordering};

use rc_game::offsets::{ITEM_MANAGER_ADD_FUNCTION_RVA, ITEM_MANAGER_RVA};
use rc_game::pickups::{is_key_item, local_key_item_notice};
use rc_proto::{ItemPickup, MAX_ITEM_COUNT, MAX_ITEM_ID, Message};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::event_log::post_event;
use crate::game_hooks::hook_address;
use crate::net;

static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
static ORIGINAL_ADD: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static SEEN_MANAGER: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

type AddItem = unsafe extern "system" fn(*mut c_void, i32, i32) -> i64;

pub fn install(build: VerifiedBuild) {
    if HOOKED_BUILD.set(build).is_err() {
        return;
    }
    #[expect(unsafe_code, reason = "hooks ItemManager::Add by its address")]
    // SAFETY: the RVA starts ItemManager::Add (manager, item id, count -> count added) in the
    // verified build, and the replacement has the same signature.
    unsafe {
        hook_address(
            "ItemManager::Add",
            (build.module_base() + u64::from(ITEM_MANAGER_ADD_FUNCTION_RVA)) as usize,
            add_item_detour as *const () as *mut c_void,
            &ORIGINAL_ADD,
        )
    };
}

#[expect(
    unsafe_code,
    reason = "calls the original ItemManager::Add through the trampoline MinHook built"
)]
pub(super) fn add_item_for_partner(item_id: u16, count: u8) -> Option<i64> {
    let original = ORIGINAL_ADD.load(Ordering::Acquire);
    let build = HOOKED_BUILD.get()?;
    if original.is_null() {
        return None;
    }
    let mut manager = SEEN_MANAGER.load(Ordering::Acquire);
    if manager.is_null() {
        manager = (build.module_base() + u64::from(ITEM_MANAGER_RVA)) as usize as *mut c_void;
    }
    // SAFETY: the trampoline continues into the original Add with its exact signature; the
    // manager is the pointer the game itself passed, or the global manager object; this runs
    // on the game thread.
    Some(unsafe {
        core::mem::transmute::<*mut c_void, AddItem>(original)(
            manager,
            i32::from(item_id),
            i32::from(count),
        )
    })
}

#[expect(
    unsafe_code,
    reason = "calls the original ItemManager::Add through the trampoline MinHook built"
)]
extern "system" fn add_item_detour(manager: *mut c_void, item_id: i32, count: i32) -> i64 {
    let original = ORIGINAL_ADD.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    SEEN_MANAGER.store(manager, Ordering::Release);
    // SAFETY: the trampoline continues into the original Add, which has this exact signature.
    let added =
        unsafe { core::mem::transmute::<*mut c_void, AddItem>(original)(manager, item_id, count) };
    if added > 0 {
        let _ = panic::catch_unwind(|| report_pickup(item_id, added));
    }
    added
}

fn report_pickup(item_id: i32, added: i64) {
    let Some(net) = net::handle() else {
        return;
    };
    let view = net.view();
    if !view.partner_in_play() {
        return;
    }
    let (Ok(item_id), Ok(count)) = (
        u16::try_from(item_id),
        u8::try_from(added.min(i64::from(MAX_ITEM_COUNT))),
    ) else {
        return;
    };
    if item_id > MAX_ITEM_ID {
        return;
    }
    let pickup = ItemPickup { item_id, count };
    net.send_relayed(Message::Pickup(pickup));
    log_line(&format!(
        "pickups: sent item id {item_id} x{count}, key item {}",
        is_key_item(item_id)
    ));
    if let Some(name) = view.peer_name
        && let Some(notice) = local_key_item_notice(&name, pickup)
    {
        post_event(&notice);
    }
}
