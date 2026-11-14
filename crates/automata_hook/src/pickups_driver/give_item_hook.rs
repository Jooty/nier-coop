use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{Mutex, PoisonError};

use automata_game::address::GIVE_ITEM_FUNCTION;
use automata_game::pickups::{GIVEN_ITEMS_QUEUED_AT_MOST, is_told_give};

use crate::game_calls::GameCalls;

static ORIGINAL_GIVE_ITEM: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static GIVEN_ITEM_IDS: Mutex<Vec<u32>> = Mutex::new(Vec::new());

type GiveItem = unsafe extern "system" fn(i32, i32, i32, i32, u32) -> i32;

/// Hooks the game's give-item call, through which the world hands the local player its items.
pub(super) fn install_give_item_hook(calls: GameCalls) -> bool {
    #[expect(
        unsafe_code,
        reason = "hooks the game's give-item call; the replacement has the signature of the original"
    )]
    // SAFETY: the address is the give-item call of the verified build whose start bytes the
    // caller checked; the replacement takes the five values the original takes and returns its result.
    unsafe {
        nt_hook::detour::hook_address(
            "give item (pickups told to the other players)",
            calls.address_value(GIVE_ITEM_FUNCTION) as usize,
            give_item_detour as *const () as *mut c_void,
            &ORIGINAL_GIVE_ITEM,
        )
    }
}

/// The ids of the items the game gave and showed since the last call, oldest first.
pub(super) fn take_given_item_ids() -> Vec<u32> {
    std::mem::take(
        &mut *GIVEN_ITEM_IDS
            .lock()
            .unwrap_or_else(PoisonError::into_inner),
    )
}

#[expect(
    unsafe_code,
    reason = "calls the original give-item call through the trampoline MinHook built"
)]
extern "system" fn give_item_detour(
    item_id: i32,
    show_log: i32,
    play_sound: i32,
    fourth: i32,
    fifth: u32,
) -> i32 {
    let original = ORIGINAL_GIVE_ITEM.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    // SAFETY: the trampoline continues into the original call, which has this exact signature.
    let added = unsafe {
        core::mem::transmute::<*mut c_void, GiveItem>(original)(
            item_id, show_log, play_sound, fourth, fifth,
        )
    };
    if is_told_give(show_log, added)
        && let Ok(item_id) = u32::try_from(item_id)
    {
        let _ = panic::catch_unwind(|| {
            let mut given = GIVEN_ITEM_IDS
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            if given.len() < GIVEN_ITEMS_QUEUED_AT_MOST {
                given.push(item_id);
            }
        });
    }
    added
}
