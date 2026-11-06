use core::ffi::c_void;
use core::ptr;
use std::collections::{HashMap, HashSet};
use std::panic;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{LazyLock, Mutex, PoisonError};

use crate::diagnostics::log_line;
use crate::game_hooks::hook_address;

const HOLD_SLOT_COUNT: usize = 12;

static ORIGINAL_UPDATES: [AtomicPtr<c_void>; HOLD_SLOT_COUNT] =
    [const { AtomicPtr::new(ptr::null_mut()) }; HOLD_SLOT_COUNT];
static HOOKED_UPDATES: LazyLock<Mutex<HashMap<usize, Option<usize>>>> =
    LazyLock::new(Mutex::default);
static HELD_ENEMIES: LazyLock<Mutex<HashSet<u64>>> = LazyLock::new(Mutex::default);
static SLOT_DETOURS: [MainUpdate; HOLD_SLOT_COUNT] = [
    hold_slot_0,
    hold_slot_1,
    hold_slot_2,
    hold_slot_3,
    hold_slot_4,
    hold_slot_5,
    hold_slot_6,
    hold_slot_7,
    hold_slot_8,
    hold_slot_9,
    hold_slot_10,
    hold_slot_11,
];

type MainUpdate = extern "system" fn(*mut c_void, f32) -> u8;
type OriginalUpdate = unsafe extern "system" fn(*mut c_void, f32) -> u8;

macro_rules! hold_slot {
    ($name:ident, $slot:expr) => {
        extern "system" fn $name(actor: *mut c_void, delta_seconds: f32) -> u8 {
            hold_or_update($slot, actor, delta_seconds)
        }
    };
}

hold_slot!(hold_slot_0, 0);
hold_slot!(hold_slot_1, 1);
hold_slot!(hold_slot_2, 2);
hold_slot!(hold_slot_3, 3);
hold_slot!(hold_slot_4, 4);
hold_slot!(hold_slot_5, 5);
hold_slot!(hold_slot_6, 6);
hold_slot!(hold_slot_7, 7);
hold_slot!(hold_slot_8, 8);
hold_slot!(hold_slot_9, 9);
hold_slot!(hold_slot_10, 10);
hold_slot!(hold_slot_11, 11);

pub(super) fn set_held(actors: HashSet<u64>) {
    *HELD_ENEMIES.lock().unwrap_or_else(PoisonError::into_inner) = actors;
}

pub(super) fn release_held(actor: u64) {
    HELD_ENEMIES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .remove(&actor);
}

pub(super) fn hold_update_function(function: usize) -> bool {
    let mut hooked = HOOKED_UPDATES
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    // A function that failed once is remembered as None and never tried again.
    if let Some(slot) = hooked.get(&function) {
        return slot.is_some();
    }
    let slot = hooked.values().flatten().count();
    if slot >= HOLD_SLOT_COUNT {
        log_line(&format!(
            "combat: no free hold slot for the enemy update at {function:#x}; that class runs its own AI"
        ));
        hooked.insert(function, None);
        return false;
    }
    #[expect(
        unsafe_code,
        reason = "hooks an enemy class's main update read from its vtable"
    )]
    // SAFETY: the address is the vslot 0x4E0 entry of a live enemy's vtable inside the game
    // image; every class's slot takes (actor, delta seconds) and returns a bool, which the slot
    // detour mirrors and passes on unchanged for actors that are not held.
    let done = unsafe {
        hook_address(
            &format!("enemy main update {function:#x}"),
            function,
            SLOT_DETOURS[slot] as *const () as *mut c_void,
            &ORIGINAL_UPDATES[slot],
        )
    };
    hooked.insert(function, done.then_some(slot));
    done
}

fn is_held(actor: u64) -> bool {
    HELD_ENEMIES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .contains(&actor)
}

#[expect(
    unsafe_code,
    reason = "calls the original main update through the trampoline MinHook built"
)]
fn hold_or_update(slot: usize, actor: *mut c_void, delta_seconds: f32) -> u8 {
    // A false return makes the actor manager destroy the actor, so a held enemy answers true.
    if panic::catch_unwind(|| is_held(actor as u64)).unwrap_or(false) {
        return 1;
    }
    let original = ORIGINAL_UPDATES[slot].load(Ordering::Acquire);
    if original.is_null() {
        return 1;
    }
    // SAFETY: the trampoline continues into the original update, which has this exact signature.
    unsafe { core::mem::transmute::<*mut c_void, OriginalUpdate>(original)(actor, delta_seconds) }
}
