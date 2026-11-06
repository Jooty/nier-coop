use core::ffi::c_void;
use core::ptr;
use std::cell::Cell;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

use rc_game::GameMemory;
use rc_game::dialogue::{
    TALK_JOIN_RANGE_UNITS, TALK_UPDATE_INPUT_BYTES, TALK_UPDATE_INPUT_ENABLED_OFFSET,
    TALK_WINDOW_IS_BUSY_RVA, TALK_WINDOW_STATE_COUNT, TALK_WINDOW_STATE_TABLE_RVA,
    TALK_WINDOW_UPDATE_RVA, UI_DRAW_PASS_RVA,
};
use rc_game::marker::UI_DIALOG_STATE_RVA;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;

pub(super) static MIRRORING: AtomicBool = AtomicBool::new(false);
static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
static ORIGINAL_IS_BUSY: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_UPDATE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_UI_DRAW_PASS: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

thread_local! {
    static IN_UI_DRAW_PASS: Cell<bool> = const { Cell::new(false) };
}

type IsBusy = unsafe extern "system" fn(*mut c_void, u8) -> u8;
type WindowStep = unsafe extern "system" fn(*mut c_void, *mut u8);
type UiDrawPass = unsafe extern "system" fn(*mut c_void);

/// Hooks the talk window's "is open" test and its per-frame update, so a window that only
/// mirrors the partner's talk neither blocks this player nor reads their buttons, and the UI
/// draw pass, which must still see the window as open to draw its text.
pub fn install(build: VerifiedBuild) {
    if HOOKED_BUILD.set(build).is_err() {
        return;
    }
    let base = build.module_base();
    #[expect(
        unsafe_code,
        reason = "hooks the talk window's busy test and update by address"
    )]
    // SAFETY: both RVAs start the named functions in the verified build (see docs
    // off-hand/dialogue.md), and each replacement has the same signature.
    unsafe {
        hook_address(
            "TalkWindow::IsBusy",
            (base + u64::from(TALK_WINDOW_IS_BUSY_RVA)) as usize,
            is_busy_detour as *const () as *mut c_void,
            &ORIGINAL_IS_BUSY,
        );
        hook_address(
            "TalkWindow::Update",
            (base + u64::from(TALK_WINDOW_UPDATE_RVA)) as usize,
            update_detour as *const () as *mut c_void,
            &ORIGINAL_UPDATE,
        );
        hook_address(
            "UiManager::DrawPass",
            (base + u64::from(UI_DRAW_PASS_RVA)) as usize,
            ui_draw_pass_detour as *const () as *mut c_void,
            &ORIGINAL_UI_DRAW_PASS,
        );
    }
    log_line(&format!(
        "dialogue: join range {TALK_JOIN_RANGE_UNITS} units (the zone rule's default; the game's own talk range was not found)"
    ));
}

#[expect(
    unsafe_code,
    reason = "calls the original TalkWindow::IsBusy through the trampoline MinHook built"
)]
extern "system" fn is_busy_detour(window: *mut c_void, script_waits: u8) -> u8 {
    // Other systems ask with 0 ("is a talk on screen?"); a mirrored talk must not stop them.
    if script_waits == 0 && MIRRORING.load(Ordering::Acquire) && !IN_UI_DRAW_PASS.with(Cell::get) {
        return 0;
    }
    let original = ORIGINAL_IS_BUSY.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    // SAFETY: the trampoline continues into the original IsBusy, which has this signature.
    unsafe { core::mem::transmute::<*mut c_void, IsBusy>(original)(window, script_waits) }
}

#[expect(
    unsafe_code,
    reason = "calls the original UI draw pass through the trampoline MinHook built"
)]
extern "system" fn ui_draw_pass_detour(ui_manager: *mut c_void) {
    let original = ORIGINAL_UI_DRAW_PASS.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    // The pass draws the talk text only when IsBusy says a talk is on screen.
    IN_UI_DRAW_PASS.with(|flag| flag.set(true));
    // SAFETY: the trampoline continues into the original pass, which has this signature.
    unsafe { core::mem::transmute::<*mut c_void, UiDrawPass>(original)(ui_manager) };
    IN_UI_DRAW_PASS.with(|flag| flag.set(false));
}

#[expect(
    unsafe_code,
    reason = "calls the talk window's update or its state handlers with the game's own arguments"
)]
extern "system" fn update_detour(window: *mut c_void, input: *mut u8) {
    if MIRRORING.load(Ordering::Acquire)
        && !input.is_null()
        && let Some(build) = HOOKED_BUILD.get()
    {
        // SAFETY: `input` is the game's own update argument, at least TALK_UPDATE_INPUT_BYTES
        // long (a stack copy the UI manager builds each frame).
        unsafe { step_mirrored_window(build, window, input) };
        return;
    }
    let original = ORIGINAL_UPDATE.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original update, which has this signature.
    unsafe { core::mem::transmute::<*mut c_void, WindowStep>(original)(window, input) }
}

#[expect(
    unsafe_code,
    reason = "copies the update argument and calls the window's state handler by address"
)]
unsafe fn step_mirrored_window(build: &VerifiedBuild, window: *mut c_void, input: *mut u8) {
    let memory = InProcessMemory::new(build);
    let base = memory.module_base();
    let menu_open = memory
        .read_i32(base + u64::from(UI_DIALOG_STATE_RVA))
        .is_ok_and(|state| state != 0);
    let Ok(state) = memory.read_i32(window as u64) else {
        return;
    };
    if menu_open || !(1..TALK_WINDOW_STATE_COUNT).contains(&state) {
        return;
    }
    let slot = base + u64::from(TALK_WINDOW_STATE_TABLE_RVA) + u64::from(state.unsigned_abs()) * 8;
    let Ok(handler) = memory.read_u64(slot) else {
        return;
    };
    let mut quiet_input = [0_u8; TALK_UPDATE_INPUT_BYTES];
    // SAFETY: the caller guarantees `input` holds TALK_UPDATE_INPUT_BYTES readable bytes.
    unsafe {
        ptr::copy_nonoverlapping(input, quiet_input.as_mut_ptr(), TALK_UPDATE_INPUT_BYTES);
    }
    // The window's button tests all require this flag, so the partner's presses never advance it.
    quiet_input[TALK_UPDATE_INPUT_ENABLED_OFFSET] = 0;
    // The original update also resets a player field each frame; the mirror leaves the player alone.
    // SAFETY: the handler is the game's own state function for this state, called with the
    // window and an update argument shaped like the game's, on the game thread.
    unsafe {
        core::mem::transmute::<usize, WindowStep>(handler as usize)(
            window,
            quiet_input.as_mut_ptr(),
        );
    }
}
