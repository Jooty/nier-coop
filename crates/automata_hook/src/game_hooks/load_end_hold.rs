use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU8, AtomicU64, Ordering};

use automata_game::address::{
    LOAD_DONE_REQUEST_VALUE, LOADING_SCREEN_ANSWER_VALUE, LOADING_SCREEN_OBJECT,
    SCENE_LOADER_FUNCTION, SCENE_LOADER_STAGE_VALUE,
};
use automata_game::exe::ExeVariant;
use automata_game::load_gate::{
    HoldReading, LOAD_DONE_NOT_REQUESTED, LOAD_DONE_REQUESTED, LOAD_END_HOLD_HARD_LIMIT_MS,
    LOADING_SCREEN_SHOWN_PART_OFFSET, LoaderAddresses, SCENE_LOADER_CODE_CHECKED_LEN,
    SCENE_LOADER_LAST_STAGE, UI_PART_ASKED_TO_PLAY_OUT_BIT, UI_PART_FLAGS_OFFSET,
    loader_code_is_the_noted_one,
};
use nc_hook::diagnostics::log_line;
use nc_hook::net::net_clock_ms;

use crate::game_calls::GameCalls;
use crate::game_hooks::boot_screen_keep::after_a_loader_step_of_the_boot;

const NOT_HELD: u8 = 0;
const HELD: u8 = 1;
const RELEASED: u8 = 2;
const ENDED_BY_THE_GAME: u8 = 3;
const ENDED_BY_THE_HARD_LIMIT: u8 = 4;

static ORIGINAL_SCENE_LOADER: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static HOLD_CALLS: OnceLock<GameCalls> = OnceLock::new();
static HOLD_WANTED: AtomicBool = AtomicBool::new(false);
static RELEASE_ASKED: AtomicBool = AtomicBool::new(false);
static HOLD_STATE: AtomicU8 = AtomicU8::new(NOT_HELD);
static HELD_SINCE_MS: AtomicU64 = AtomicU64::new(0);

type SceneLoader = unsafe extern "system" fn() -> u64;

/// Hooks the game's scene loader so the end of a load can be kept back. True when the loader of
/// this exe is the noted one and the hook is in place; otherwise nothing is touched.
pub fn install_load_end_hold(calls: GameCalls, variant: ExeVariant) -> bool {
    let addresses = LoaderAddresses {
        loader_function: SCENE_LOADER_FUNCTION.rva(variant),
        stage_value: SCENE_LOADER_STAGE_VALUE.rva(variant),
        load_done_request_value: LOAD_DONE_REQUEST_VALUE.rva(variant),
        loading_screen_answer_value: LOADING_SCREEN_ANSWER_VALUE.rva(variant),
    };
    let code_is_the_noted_one = calls
        .read_bytes(SCENE_LOADER_FUNCTION, SCENE_LOADER_CODE_CHECKED_LEN)
        .is_ok_and(|code| loader_code_is_the_noted_one(&code, addresses));
    if !code_is_the_noted_one {
        log_line(
            "start: the game's scene loader is not the noted one on this exe, the end of a load is never held",
        );
        return false;
    }
    if HOLD_CALLS.set(calls).is_err() {
        return false;
    }
    #[expect(
        unsafe_code,
        reason = "hooks the game's scene loader; the replacement has the signature of the original"
    )]
    // SAFETY: the address is the loader of the verified build and its code was just checked; it
    // takes no arguments and gives one value back, like the replacement.
    unsafe {
        nc_hook::detour::hook_address(
            "scene loader (the end of a load can be held)",
            calls.address_value(SCENE_LOADER_FUNCTION) as usize,
            scene_loader_detour as *const () as *mut c_void,
            &ORIGINAL_SCENE_LOADER,
        )
    }
}

/// While wanted, the next load that finishes stays on its loading screen; taking the wish back
/// lets a held load end.
pub fn set_load_end_hold_wanted(wanted: bool) {
    HOLD_WANTED.store(wanted, Ordering::Release);
}

pub fn release_load_end_hold() {
    RELEASE_ASKED.store(true, Ordering::Release);
}

pub fn load_end_hold_reading() -> HoldReading {
    match HOLD_STATE.load(Ordering::Acquire) {
        HELD => HoldReading::Held,
        RELEASED => HoldReading::Released,
        ENDED_BY_THE_GAME => HoldReading::EndedByTheGame,
        ENDED_BY_THE_HARD_LIMIT => HoldReading::EndedByTheHardLimit,
        _ => HoldReading::NotHeld,
    }
}

fn after_a_loader_step(calls: &GameCalls, loader_finished: bool) {
    if after_a_loader_step_of_the_boot(calls, loader_finished) {
        return;
    }
    match HOLD_STATE.load(Ordering::Acquire) {
        NOT_HELD => catch_the_end_of_the_load(calls, loader_finished),
        HELD => keep_or_end_the_hold(calls, loader_finished),
        _ => {}
    }
}

fn catch_the_end_of_the_load(calls: &GameCalls, loader_finished: bool) {
    if loader_finished || !HOLD_WANTED.load(Ordering::Acquire) {
        return;
    }
    // The loader's fourth stage has just asked the loading screen to go away. Taking the request
    // back before the loading screen runs keeps the screen up and the loader in its last stage.
    let asked_just_now = calls.read_i32(SCENE_LOADER_STAGE_VALUE) == Ok(SCENE_LOADER_LAST_STAGE)
        && calls.read_i32(LOAD_DONE_REQUEST_VALUE) == Ok(LOAD_DONE_REQUESTED)
        && calls.read_i32(LOADING_SCREEN_ANSWER_VALUE) == Ok(0);
    if !asked_just_now {
        return;
    }
    if calls
        .write_i32(LOAD_DONE_REQUEST_VALUE, LOAD_DONE_NOT_REQUESTED)
        .is_err()
    {
        return;
    }
    RELEASE_ASKED.store(false, Ordering::Release);
    HELD_SINCE_MS.store(net_clock_ms(), Ordering::Release);
    HOLD_STATE.store(HELD, Ordering::Release);
    log_line("start: the load is done; its end is held, the loading screen stays");
}

fn keep_or_end_the_hold(calls: &GameCalls, loader_finished: bool) {
    let stage_is_the_last = calls.read_i32(SCENE_LOADER_STAGE_VALUE) == Ok(SCENE_LOADER_LAST_STAGE);
    if loader_finished || !stage_is_the_last {
        // The game went on by a way of its own: leave what it would have left behind.
        let _ = calls.write_i32(LOAD_DONE_REQUEST_VALUE, LOAD_DONE_REQUESTED);
        HOLD_STATE.store(ENDED_BY_THE_GAME, Ordering::Release);
        log_line("start: the game ended the load although its end was held");
        return;
    }
    ask_the_loading_screen_to_play_out(calls);
    let held_ms = net_clock_ms().saturating_sub(HELD_SINCE_MS.load(Ordering::Acquire));
    let over_the_hard_limit = held_ms >= LOAD_END_HOLD_HARD_LIMIT_MS;
    let let_go = RELEASE_ASKED.load(Ordering::Acquire) || !HOLD_WANTED.load(Ordering::Acquire);
    if !let_go && !over_the_hard_limit {
        return;
    }
    let _ = calls.write_i32(LOAD_DONE_REQUEST_VALUE, LOAD_DONE_REQUESTED);
    if let_go {
        HOLD_STATE.store(RELEASED, Ordering::Release);
        log_line(&format!(
            "start: the held end of the load is let go after {held_ms} ms"
        ));
    } else {
        HOLD_STATE.store(ENDED_BY_THE_HARD_LIMIT, Ordering::Release);
        log_line(&format!(
            "start: the end of the load was held for {held_ms} ms, the hard limit; it is let go"
        ));
    }
}

fn ask_the_loading_screen_to_play_out(calls: &GameCalls) {
    // The loading screen sets this bit itself on the request that is held back here. Set early,
    // the screen is through with what it shows before the release, on both PCs alike.
    let screen = calls.address_value(LOADING_SCREEN_OBJECT);
    let Ok(part) = calls.read_u64_at(screen + u64::from(LOADING_SCREEN_SHOWN_PART_OFFSET)) else {
        return;
    };
    if part == 0 {
        return;
    }
    let flags_at = part + u64::from(UI_PART_FLAGS_OFFSET);
    if let Ok(flags) = calls.read_u32_at(flags_at)
        && flags & UI_PART_ASKED_TO_PLAY_OUT_BIT == 0
    {
        let _ = calls.write_u32_at(flags_at, flags | UI_PART_ASKED_TO_PLAY_OUT_BIT);
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original scene loader through the trampoline MinHook built"
)]
extern "system" fn scene_loader_detour() -> u64 {
    let original = ORIGINAL_SCENE_LOADER.load(Ordering::Acquire);
    if original.is_null() {
        // Zero is the loader's own "not done yet": the scene task calls again next frame.
        return 0;
    }
    // SAFETY: the trampoline continues into the original loader, which takes no arguments.
    let finished = unsafe { core::mem::transmute::<*mut c_void, SceneLoader>(original)() };
    if let Some(calls) = HOLD_CALLS.get() {
        let _ = panic::catch_unwind(|| after_a_loader_step(calls, finished as u32 != 0));
    }
    finished
}
