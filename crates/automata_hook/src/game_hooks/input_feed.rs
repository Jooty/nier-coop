use core::ffi::c_void;
use core::ptr;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, AtomicUsize, Ordering};

use automata_game::address::{
    CONTROLLER_UPDATE_INPUT_CALLS, PROCESS_BUTTONS_FUNCTION, STICK_UPDATE_FUNCTION,
};
use automata_game::autopilot::StickFeed;
use automata_game::exe::ExeVariant;
use automata_game::partner::{
    BUTTON_WORD_COUNT, CONTROLLER_BUTTON_WORDS_OFFSET, CONTROLLER_CALLS_LEN,
    CONTROLLER_HELD_BUTTONS_OFFSET, CONTROLLER_PRESS_TIMER_START_OFFSET, CONTROLLER_SPEED_OFFSET,
    CONTROLLER_WORLD_FACING_OFFSET, InputFeed, PROCESS_BUTTONS_CHECKED_LEN, PartnerDrive,
    button_words_after_presses, controller_calls_match, process_buttons_stores_match,
};
use nc_hook::diagnostics::log_line;
use nc_hook::memory::MemoryError;

use crate::game_calls::GameCalls;

const NO_FED_CONTROLLER: usize = 0;

static ORIGINAL_PROCESS_BUTTONS: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_STICK_UPDATE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static FEED_CALLS: OnceLock<GameCalls> = OnceLock::new();
static FED_CONTROLLER: AtomicUsize = AtomicUsize::new(NO_FED_CONTROLLER);
static FEED_HELD: AtomicU32 = AtomicU32::new(0);
static FEED_PRESSED: AtomicU32 = AtomicU32::new(0);
static FEED_SPEED_BITS: AtomicU32 = AtomicU32::new(0);
static FEED_WORLD_FACING_BITS: AtomicU32 = AtomicU32::new(0);
static FIRST_BUTTON_FEED_LOGGED: AtomicBool = AtomicBool::new(false);
static FIRST_STICK_FEED_LOGGED: AtomicBool = AtomicBool::new(false);
static AUTOPILOT_CONTROLLER: AtomicUsize = AtomicUsize::new(NO_FED_CONTROLLER);
static AUTOPILOT_SPEED_BITS: AtomicU32 = AtomicU32::new(0);
static AUTOPILOT_WORLD_FACING_BITS: AtomicU32 = AtomicU32::new(0);
static FIRST_AUTOPILOT_FEED_LOGGED: AtomicBool = AtomicBool::new(false);

type ProcessButtons = unsafe extern "system" fn(usize);
type StickUpdate = unsafe extern "system" fn(usize, usize);

/// Which of the two controller functions are hooked whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FeedHooks {
    pub buttons: bool,
    pub stick: bool,
}

/// Hooks the stick update for speed and facing in both drives, and the button processing for
/// input drive; nothing is hooked where the code differs from the noted one.
pub fn install_input_feed(
    calls: GameCalls,
    variant: ExeVariant,
    wanted: PartnerDrive,
) -> FeedHooks {
    let checks = code_checks(&calls, variant);
    if !checks.calls_match {
        log_line(
            "input feed: the controller's two calls are not the noted ones, nothing hooked; the partner is driven by state only and gets no speed and no facing",
        );
        return FeedHooks::default();
    }
    if FEED_CALLS.set(calls).is_err() {
        return FeedHooks::default();
    }
    #[expect(
        unsafe_code,
        reason = "hooks the controller's stick update; the replacement has the signature of the original"
    )]
    // SAFETY: the address passed the call-site check for this function of the verified build;
    // the replacement takes the same integer arguments and calls the original first.
    let stick = unsafe {
        nc_hook::detour::hook_address(
            "controller stick update (speed and facing feed, whole function)",
            calls.address_value(STICK_UPDATE_FUNCTION) as usize,
            stick_update_detour as *const () as *mut c_void,
            &ORIGINAL_STICK_UPDATE,
        )
    };
    if !stick {
        log_line(
            "input feed: the stick update hook failed; the partner is driven by state only and gets no speed and no facing",
        );
        return FeedHooks::default();
    }
    if wanted != PartnerDrive::Input {
        return FeedHooks {
            buttons: false,
            stick,
        };
    }
    if !checks.stores_match {
        log_line(
            "input feed: the button processing is not the noted one, it is not hooked; the partner is driven by state with speed and facing fed",
        );
        return FeedHooks {
            buttons: false,
            stick,
        };
    }
    #[expect(
        unsafe_code,
        reason = "hooks the controller's button processing; the replacement has the signature of the original"
    )]
    // SAFETY: the address passed the byte checks for this function of the verified build; the
    // replacement takes the same integer argument and calls the original first.
    let buttons = unsafe {
        nc_hook::detour::hook_address(
            "controller button processing (input feed, whole function)",
            calls.address_value(PROCESS_BUTTONS_FUNCTION) as usize,
            process_buttons_detour as *const () as *mut c_void,
            &ORIGINAL_PROCESS_BUTTONS,
        )
    };
    if !buttons {
        log_line(
            "input feed: the button processing hook failed; the partner is driven by state with speed and facing fed",
        );
    }
    FeedHooks { buttons, stick }
}

/// Sets what the next controller update of this controller gets; pressed bits add up until fed.
pub fn set_input_feed(controller: usize, feed: InputFeed) {
    FEED_HELD.store(feed.held, Ordering::Relaxed);
    FEED_PRESSED.fetch_or(feed.pressed, Ordering::Relaxed);
    FEED_SPEED_BITS.store(feed.speed.to_bits(), Ordering::Relaxed);
    FEED_WORLD_FACING_BITS.store(feed.world_facing.to_bits(), Ordering::Relaxed);
    FED_CONTROLLER.store(controller, Ordering::Release);
}

pub fn clear_input_feed() {
    FED_CONTROLLER.store(NO_FED_CONTROLLER, Ordering::Release);
    FEED_PRESSED.store(0, Ordering::Relaxed);
}

/// Sets what the stick update of the local hero's controller is overwritten with.
pub fn set_autopilot_feed(controller: usize, feed: StickFeed) {
    AUTOPILOT_SPEED_BITS.store(feed.speed.to_bits(), Ordering::Relaxed);
    AUTOPILOT_WORLD_FACING_BITS.store(feed.world_facing.to_bits(), Ordering::Relaxed);
    AUTOPILOT_CONTROLLER.store(controller, Ordering::Release);
}

pub fn clear_autopilot_feed() {
    AUTOPILOT_CONTROLLER.store(NO_FED_CONTROLLER, Ordering::Release);
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
    if controller != NO_FED_CONTROLLER && controller == AUTOPILOT_CONTROLLER.load(Ordering::Acquire)
    {
        feed_autopilot(controller);
    }
}

struct CodeChecks {
    calls_match: bool,
    stores_match: bool,
}

fn code_checks(calls: &GameCalls, variant: ExeVariant) -> CodeChecks {
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
    CodeChecks {
        calls_match,
        stores_match,
    }
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
    let base = controller as u64;
    let written = calls
        .write_u32s_at(
            base + u64::from(CONTROLLER_HELD_BUTTONS_OFFSET),
            &[held, pressed],
        )
        .and_then(|()| start_press_timers(calls, base, pressed));
    if !FIRST_BUTTON_FEED_LOGGED.swap(true, Ordering::Relaxed) {
        log_line(&format!(
            "input feed: first button write after the partner's button processing (held {held:#X}, pressed {pressed:#X}, written {})",
            written.is_ok()
        ));
    }
}

fn start_press_timers(calls: &GameCalls, controller: u64, pressed: u32) -> Result<(), MemoryError> {
    let words_address = controller + u64::from(CONTROLLER_BUTTON_WORDS_OFFSET);
    // The original has just counted the eight timers down; a press only starts its own.
    let counted_down: [u32; BUTTON_WORD_COUNT] = calls.read_u32s_at(words_address)?;
    let timer_start =
        calls.read_u32_at(controller + u64::from(CONTROLLER_PRESS_TIMER_START_OFFSET))?;
    let words = button_words_after_presses(counted_down, pressed, timer_start);
    if words == counted_down {
        return Ok(());
    }
    calls.write_u32s_at(words_address, &words)
}

fn feed_autopilot(controller: usize) {
    let Some(calls) = FEED_CALLS.get() else {
        return;
    };
    let speed = f32::from_bits(AUTOPILOT_SPEED_BITS.load(Ordering::Relaxed));
    let world_facing = f32::from_bits(AUTOPILOT_WORLD_FACING_BITS.load(Ordering::Relaxed));
    let written = write_speed_and_facing(calls, controller, speed, world_facing);
    if !FIRST_AUTOPILOT_FEED_LOGGED.swap(true, Ordering::Relaxed) {
        log_line(&format!(
            "autopilot: first speed and facing write after the local hero's stick update (speed {speed}, world facing {world_facing}, written {}; that the hero walks from these two fields alone is a guess until run)",
            written.is_ok()
        ));
    }
}

fn write_speed_and_facing(
    calls: &GameCalls,
    controller: usize,
    speed: f32,
    world_facing: f32,
) -> Result<(), MemoryError> {
    let base = controller as u64;
    calls.write_f32s_at(
        base + u64::from(CONTROLLER_SPEED_OFFSET),
        &[speed, speed * speed],
    )?;
    calls.write_f32_at(
        base + u64::from(CONTROLLER_WORLD_FACING_OFFSET),
        world_facing,
    )
}

fn feed_stick(controller: usize) {
    let Some(calls) = FEED_CALLS.get() else {
        return;
    };
    let speed = f32::from_bits(FEED_SPEED_BITS.load(Ordering::Relaxed));
    let world_facing = f32::from_bits(FEED_WORLD_FACING_BITS.load(Ordering::Relaxed));
    let written = write_speed_and_facing(calls, controller, speed, world_facing);
    if !FIRST_STICK_FEED_LOGGED.swap(true, Ordering::Relaxed) {
        log_line(&format!(
            "input feed: first speed and facing write after the partner's stick update (speed {speed}, world facing {world_facing}, written {})",
            written.is_ok()
        ));
    }
}
