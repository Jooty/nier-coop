use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{OnceLock, PoisonError};

use rc_game::GameMemory;
use rc_game::cutscene_skip::{
    CONFIRM_KEYBOARD_OBJECT_RVA, CONFIRM_KEYBOARD_PRESSED_RVA, CONFIRM_PAD_BUTTON_MASK,
    CONFIRM_PAD_MODE, CONFIRM_PAD_OBJECT_RVA, CONFIRM_PAD_PRESSED_RVA, MAIN_MENU_IS_OPEN_RVA,
    MAIN_MENU_STATE_RVA, SKIP_CONFIRM_WINDOW_BELOW_SECONDS, SKIP_COUNTDOWN_IDLE,
    SKIP_MANAGER_ALLOWED_OFFSET, SKIP_MANAGER_ARMED_OFFSET, SKIP_MANAGER_CONFIRMED_OFFSET,
    SKIP_MANAGER_COUNTDOWN_OFFSET, SKIP_MANAGER_LABEL_OFFSET, SKIP_MODAL_IS_SHOWN_RVA,
    SKIP_MODAL_OBJECT_RVA, SKIP_PROMPT_HIDE_RVA, SKIP_PROMPT_UI_RVA, SKIP_UPDATE_RVA, SkipVerdict,
    TRIGGER_FIND_NAMED_EVENT_RVA, TRIGGER_MANAGER_RVA, TRIGGER_MARK_READY_RVA,
};

use crate::build_gate::VerifiedBuild;
use crate::cutscene_skip::state::{
    TRACKER, is_connected, now_ms, observe_scene, partner_in_cutscene, partner_name, send_ask,
};
use crate::diagnostics::log_line;
use crate::event_log::post_event;
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;
use crate::net;

static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
static ORIGINAL_UPDATE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

type SkipUpdate = unsafe extern "system" fn(*mut c_void, f32);
type ObjectTest = unsafe extern "system" fn(*mut c_void) -> u8;
type PadPressed = unsafe extern "system" fn(*mut c_void, u32, u64, i32) -> u8;
type PromptHide = unsafe extern "system" fn(*mut c_void, i32);
type FindNamedEvent = unsafe extern "system" fn(*mut c_void, *const u8, i32, i32) -> u64;
type MarkReady = unsafe extern "system" fn(*mut c_void, u64);

pub fn install(build: VerifiedBuild) {
    if HOOKED_BUILD.set(build).is_err() {
        return;
    }
    crate::cutscene_skip::start_hold::install(build);
    let base = build.module_base();
    #[expect(
        unsafe_code,
        reason = "hooks the event skip manager's update by address"
    )]
    // SAFETY: the RVA starts EventSkipManager::Update (manager, dt) in the verified build (see
    // docs off-hand/cutscenes.md) and the replacement has the same signature.
    unsafe {
        hook_address(
            "EventSkipManager::Update",
            (base + u64::from(SKIP_UPDATE_RVA)) as usize,
            update_detour as *const () as *mut c_void,
            &ORIGINAL_UPDATE,
        );
    }
}

extern "system" fn update_detour(manager: *mut c_void, dt: f32) {
    let verdict = panic::catch_unwind(|| decide(manager, dt)).unwrap_or(SkipVerdict::PassThrough);
    match verdict {
        SkipVerdict::PassThrough | SkipVerdict::LetGameSkip => call_original(manager, dt),
        SkipVerdict::Hold => {}
        SkipVerdict::SkipNow => {
            let _ = panic::catch_unwind(|| skip_now(manager));
        }
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original skip update through the trampoline MinHook built"
)]
fn call_original(manager: *mut c_void, dt: f32) {
    let original = ORIGINAL_UPDATE.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original update, which has this signature.
    unsafe { core::mem::transmute::<*mut c_void, SkipUpdate>(original)(manager, dt) };
}

fn decide(manager: *mut c_void, dt: f32) -> SkipVerdict {
    let Some(build) = HOOKED_BUILD.get() else {
        return SkipVerdict::PassThrough;
    };
    let Some(net) = net::handle() else {
        return SkipVerdict::PassThrough;
    };
    let Some((in_scene, skip_taken)) = observe_scene(build) else {
        return SkipVerdict::PassThrough;
    };
    let view = net.view();
    if !in_scene || skip_taken || !is_connected(&view) || manager.is_null() {
        return SkipVerdict::PassThrough;
    }
    let pressed = confirm_pressed(build, manager, dt);
    let participates = partner_in_cutscene(&view);
    let step = TRACKER.lock().unwrap_or_else(PoisonError::into_inner).step(
        pressed,
        participates,
        now_ms(),
    );
    if step.newly_asked {
        send_ask(net);
        log_line(&format!(
            "cutscene skip: local player asks (partner in a cutscene {participates}, verdict {:?})",
            step.verdict
        ));
        if step.verdict == SkipVerdict::Hold {
            post_event(&format!(
                "You want to skip; waiting for {}",
                partner_name(&view)
            ));
        }
    } else if step.verdict == SkipVerdict::SkipNow {
        log_line("cutscene skip: everyone asked, skipping together");
    }
    step.verdict
}

#[expect(
    unsafe_code,
    reason = "reads the skip manager's fields and calls the game's own input tests"
)]
fn confirm_pressed(build: &VerifiedBuild, manager: *mut c_void, dt: f32) -> bool {
    let base = build.module_base();
    let bytes = manager.cast::<u8>();
    // SAFETY: `manager` is the skip manager the game passed to its own update; every offset read
    // is inside its 0x110-byte layout.
    let (armed, confirmed, allowed, countdown) = unsafe {
        (
            bytes.add(SKIP_MANAGER_ARMED_OFFSET as usize).read(),
            bytes.add(SKIP_MANAGER_CONFIRMED_OFFSET as usize).read(),
            bytes.add(SKIP_MANAGER_ALLOWED_OFFSET as usize).read(),
            bytes
                .add(SKIP_MANAGER_COUNTDOWN_OFFSET as usize)
                .cast::<f32>()
                .read_unaligned(),
        )
    };
    if armed != 1 || confirmed != 0 || allowed == 0 || countdown < 0.0 {
        return false;
    }
    let remaining = countdown - dt;
    let in_window = if remaining <= 0.0 {
        SKIP_COUNTDOWN_IDLE
    } else {
        remaining
    };
    if in_window >= SKIP_CONFIRM_WINDOW_BELOW_SECONDS {
        return false;
    }
    let menu_state = InProcessMemory::new(build)
        .read_i32(base + u64::from(MAIN_MENU_STATE_RVA))
        .unwrap_or(0);
    if menu_state != 0 {
        return false;
    }
    // SAFETY: each RVA starts the named game function in the verified build and each object
    // address is the game object the original update passes to it; this runs on the game thread.
    unsafe {
        let menu_open = core::mem::transmute::<u64, ObjectTest>(
            base + u64::from(MAIN_MENU_IS_OPEN_RVA),
        )((base + u64::from(MAIN_MENU_STATE_RVA)) as *mut c_void);
        let modal = core::mem::transmute::<u64, ObjectTest>(
            base + u64::from(SKIP_MODAL_IS_SHOWN_RVA),
        )((base + u64::from(SKIP_MODAL_OBJECT_RVA)) as *mut c_void);
        if menu_open != 0 || modal != 0 {
            return false;
        }
        let pad =
            core::mem::transmute::<u64, PadPressed>(base + u64::from(CONFIRM_PAD_PRESSED_RVA))(
                (base + u64::from(CONFIRM_PAD_OBJECT_RVA)) as *mut c_void,
                CONFIRM_PAD_BUTTON_MASK,
                CONFIRM_PAD_MODE,
                -1,
            );
        let keyboard = core::mem::transmute::<u64, ObjectTest>(
            base + u64::from(CONFIRM_KEYBOARD_PRESSED_RVA),
        )((base + u64::from(CONFIRM_KEYBOARD_OBJECT_RVA)) as *mut c_void);
        pad != 0 || keyboard != 0
    }
}

#[expect(
    unsafe_code,
    reason = "does what the skip update does after a confirm, by writing its fields and calling its helpers"
)]
fn skip_now(manager: *mut c_void) {
    let Some(build) = HOOKED_BUILD.get() else {
        return;
    };
    let base = build.module_base();
    let bytes = manager.cast::<u8>();
    let prompt = (base + u64::from(SKIP_PROMPT_UI_RVA)) as *mut c_void;
    let hide = base + u64::from(SKIP_PROMPT_HIDE_RVA);
    // SAFETY: `manager` is the game's skip manager (0x110 bytes) and the calls are the ones the
    // original update makes on a confirm, with the same arguments, on the game thread.
    unsafe {
        bytes.add(SKIP_MANAGER_CONFIRMED_OFFSET as usize).write(1);
        core::mem::transmute::<u64, PromptHide>(hide)(prompt, 0);
        if bytes.add(SKIP_MANAGER_LABEL_OFFSET as usize).read() == 0 {
            return;
        }
        let triggers = (base + u64::from(TRIGGER_MANAGER_RVA)) as *mut c_void;
        let label = bytes.add(SKIP_MANAGER_LABEL_OFFSET as usize).cast_const();
        let found = core::mem::transmute::<u64, FindNamedEvent>(
            base + u64::from(TRIGGER_FIND_NAMED_EVENT_RVA),
        )(triggers, label, 0, 0);
        if found == 0 {
            return;
        }
        core::mem::transmute::<u64, MarkReady>(base + u64::from(TRIGGER_MARK_READY_RVA))(
            triggers, found,
        );
        bytes.add(SKIP_MANAGER_ARMED_OFFSET as usize).write(0);
        bytes
            .add(SKIP_MANAGER_COUNTDOWN_OFFSET as usize)
            .cast::<f32>()
            .write_unaligned(SKIP_COUNTDOWN_IDLE);
        bytes.add(SKIP_MANAGER_CONFIRMED_OFFSET as usize).write(0);
        bytes.add(SKIP_MANAGER_LABEL_OFFSET as usize).write(0);
        core::mem::transmute::<u64, PromptHide>(hide)(prompt, 0);
    }
}
