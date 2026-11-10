use core::ffi::c_void;
use core::ptr;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, AtomicUsize, Ordering};

use automata_game::address::{
    CONTROLLER_UPDATE_INPUT_CALLS, PROCESS_BUTTONS_FUNCTION, STICK_UPDATE_FUNCTION,
};
use automata_game::exe::ExeVariant;
use automata_game::partner::{
    CONTROLLER_BUTTON_WORDS_OFFSET, CONTROLLER_CALLS_LEN, CONTROLLER_HELD_BUTTONS_OFFSET,
    CONTROLLER_SPEED_OFFSET, CONTROLLER_WORLD_FACING_OFFSET, InputFeed,
    PROCESS_BUTTONS_CHECKED_LEN, button_words_of, controller_calls_match,
    process_buttons_stores_match,
};
use nc_hook::diagnostics::log_line;

use crate::game_calls::GameCalls;

const NO_FED_CONTROLLER: usize = 0;

static ORIGINAL_PROCESS_BUTTONS: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_STICK_UPDATE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static FEED_CALLS: OnceLock<GameCalls> = OnceLock::new();
static FED_CONTROLLER: AtomicUsize = AtomicUsize::new(NO_FED_CONTROLLER);
static FEED_HELD: AtomicU32 = AtomicU32::new(0);
static FEED_PRESSED: AtomicU32 = AtomicU32::new(0);
static FEED_BUTTON_MASK: AtomicU32 = AtomicU32::new(0);
static FEED_SPEED_BITS: AtomicU32 = AtomicU32::new(0);
static FEED_WORLD_FACING_BITS: AtomicU32 = AtomicU32::new(0);
static FIRST_BUTTON_FEED_LOGGED: AtomicBool = AtomicBool::new(false);
static FIRST_STICK_FEED_LOGGED: AtomicBool = AtomicBool::new(false);

type ProcessButtons = unsafe extern "system" fn(usize);
type StickUpdate = unsafe extern "system" fn(usize, usize);

/// Hooks the two controller functions whole; false, with nothing hooked, when the code differs.
pub fn install_input_feed(calls: GameCalls, variant: ExeVariant) -> bool {
    if !code_is_the_noted_one(&calls, variant) {
        log_line(
            "input feed: the controller code is not the noted one, nothing hooked; the partner is driven by state only",
        );
        return false;
    }
    if FEED_CALLS.set(calls).is_err() {
        return false;
    }
    #[expect(
        unsafe_code,
        reason = "hooks two controller functions; each replacement has the signature of its original"
    )]
    // SAFETY: both addresses passed the byte checks for these functions of the verified build;
    // the replacements take the same integer arguments and call the original first.
    let hooked = unsafe {
        nc_hook::detour::hook_address(
            "controller button processing (input feed, whole function)",
            calls.address_value(PROCESS_BUTTONS_FUNCTION) as usize,
            process_buttons_detour as *const () as *mut c_void,
            &ORIGINAL_PROCESS_BUTTONS,
        ) && nc_hook::detour::hook_address(
            "controller stick update (input feed, whole function)",
            calls.address_value(STICK_UPDATE_FUNCTION) as usize,
            stick_update_detour as *const () as *mut c_void,
            &ORIGINAL_STICK_UPDATE,
        )
    };
    if !hooked {
        log_line("input feed: a hook failed; the partner is driven by state only");
    }
    hooked
}

/// Sets what the next controller update of this controller gets; pressed bits add up until fed.
pub fn set_input_feed(controller: usize, feed: InputFeed) {
    FEED_HELD.store(feed.held, Ordering::Relaxed);
    FEED_PRESSED.fetch_or(feed.pressed, Ordering::Relaxed);
    FEED_BUTTON_MASK.store(u32::from(feed.button_mask), Ordering::Relaxed);
    FEED_SPEED_BITS.store(feed.speed.to_bits(), Ordering::Relaxed);
    FEED_WORLD_FACING_BITS.store(feed.world_facing.to_bits(), Ordering::Relaxed);
    FED_CONTROLLER.store(controller, Ordering::Release);
}

pub fn clear_input_feed() {
    FED_CONTROLLER.store(NO_FED_CONTROLLER, Ordering::Release);
    FEED_PRESSED.store(0, Ordering::Relaxed);
}

#[expect(
    unsafe_code,
    reason = "calls the original button processing through the trampoline MinHook built"
)]
extern "system" fn process_buttons_detour(controller: usize) {
    let original = ORIGINAL_PROCESS_BUTTONS.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original function with the argument received.
    unsafe { core::mem::transmute::<*mut c_void, ProcessButtons>(original)(controller) };
    if is_fed(controller) {
        feed_buttons(controller);
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original stick update through the trampoline MinHook built"
)]
extern "system" fn stick_update_detour(controller: usize, reference: usize) {
    let original = ORIGINAL_STICK_UPDATE.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original function with the arguments received.
    unsafe { core::mem::transmute::<*mut c_void, StickUpdate>(original)(controller, reference) };
    if is_fed(controller) {
        feed_stick(controller);
    }
}

fn code_is_the_noted_one(calls: &GameCalls, variant: ExeVariant) -> bool {
    let calls_found = calls
        .read_bytes_at(
            calls.address_value(CONTROLLER_UPDATE_INPUT_CALLS),
            CONTROLLER_CALLS_LEN,
        )
        .ok()
        .and_then(|bytes| <[u8; CONTROLLER_CALLS_LEN]>::try_from(bytes).ok());
    let calls_match = calls_found.is_some_and(|found| {
        controller_calls_match(
            &found,
            CONTROLLER_UPDATE_INPUT_CALLS.rva(variant),
            PROCESS_BUTTONS_FUNCTION.rva(variant),
            STICK_UPDATE_FUNCTION.rva(variant),
        )
    });
    let stores_match = calls
        .read_bytes_at(
            calls.address_value(PROCESS_BUTTONS_FUNCTION),
            PROCESS_BUTTONS_CHECKED_LEN,
        )
        .is_ok_and(|start| process_buttons_stores_match(&start));
    calls_match && stores_match
}

fn is_fed(controller: usize) -> bool {
    controller != NO_FED_CONTROLLER && controller == FED_CONTROLLER.load(Ordering::Acquire)
}

fn feed_buttons(controller: usize) {
    let Some(calls) = FEED_CALLS.get() else {
        return;
    };
    let held = FEED_HELD.load(Ordering::Relaxed);
    // Taking the pressed bits here hands each press to exactly one controller update.
    let pressed = FEED_PRESSED.swap(0, Ordering::Relaxed);
    let words = button_words_of(FEED_BUTTON_MASK.load(Ordering::Relaxed) as u8);
    let base = controller as u64;
    let written = calls
        .write_u32s_at(
            base + u64::from(CONTROLLER_HELD_BUTTONS_OFFSET),
            &[held, pressed],
        )
        .and_then(|()| {
            calls.write_u32s_at(base + u64::from(CONTROLLER_BUTTON_WORDS_OFFSET), &words)
        });
    if !FIRST_BUTTON_FEED_LOGGED.swap(true, Ordering::Relaxed) {
        log_line(&format!(
            "input feed: first button write after the partner's button processing (held {held:#X}, pressed {pressed:#X}, written {})",
            written.is_ok()
        ));
    }
}

fn feed_stick(controller: usize) {
    let Some(calls) = FEED_CALLS.get() else {
        return;
    };
    let speed = f32::from_bits(FEED_SPEED_BITS.load(Ordering::Relaxed));
    let world_facing = f32::from_bits(FEED_WORLD_FACING_BITS.load(Ordering::Relaxed));
    let base = controller as u64;
    let written = calls
        .write_f32s_at(
            base + u64::from(CONTROLLER_SPEED_OFFSET),
            &[speed, speed * speed],
        )
        .and_then(|()| {
            calls.write_f32_at(
                base + u64::from(CONTROLLER_WORLD_FACING_OFFSET),
                world_facing,
            )
        });
    if !FIRST_STICK_FEED_LOGGED.swap(true, Ordering::Relaxed) {
        log_line(&format!(
            "input feed: first speed and facing write after the partner's stick update (speed {speed}, world facing {world_facing}, written {})",
            written.is_ok()
        ));
    }
}
