use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

use rc_game::GameMemory;
use rc_game::coop_pause::{PauseView, effective_control_mask};
use rc_game::marker::{
    CONTROL_MASK_FIRST_RVA, CONTROL_MASK_SLOT_COUNT, EVENT_SCENE_BYTE_RVA, UI_DIALOG_STATE_RVA,
};
use rc_game::offsets::GET_CONTROL_MASK_FUNCTION_RVA;
use rc_game::ui_scenes::{OpenUiScenes, read_open_ui_scenes};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::hook_slot::hook_address;
use crate::memory::InProcessMemory;

static ORIGINAL_GET_CONTROL_MASK: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static MODULE_BASE: AtomicU64 = AtomicU64::new(0);
static PARTNER_CONNECTED: AtomicBool = AtomicBool::new(false);
static PAUSE_MENU_OPEN: AtomicBool = AtomicBool::new(false);
static APPLYING: AtomicBool = AtomicBool::new(false);
static LAST_MENU_SIGNALS: Mutex<Option<MenuSignals>> = Mutex::new(None);

type GetControlMask = unsafe extern "system" fn(*mut c_void) -> u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MenuSignals {
    scenes: OpenUiScenes,
    ui_dialog_state: i32,
}

/// Runs once per script tick on the game thread: reads the UI scene list once, so the control
/// mask getter (called many times a frame) only reads two flags.
pub fn update_menu_no_pause(build: &VerifiedBuild, partner_connected: bool) {
    PARTNER_CONNECTED.store(partner_connected, Ordering::Relaxed);
    let memory = InProcessMemory::new(build);
    let base = memory.module_base();
    let scenes = read_open_ui_scenes(&memory).unwrap_or_default();
    PAUSE_MENU_OPEN.store(scenes.pause_menu_open(), Ordering::Relaxed);
    let signals = MenuSignals {
        scenes,
        ui_dialog_state: memory
            .read_i32(base + u64::from(UI_DIALOG_STATE_RVA))
            .unwrap_or(0),
    };
    let mut last = LAST_MENU_SIGNALS
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if *last == Some(signals) {
        return;
    }
    *last = Some(signals);
    let menu_slot = memory
        .read_i32(base + u64::from(CONTROL_MASK_FIRST_RVA))
        .unwrap_or(0);
    log_line(&format!(
        "menu signals: ui scenes {:?}, short system menu open {}, pause menu open {}, ui dialog state {:#x}, mask slot 0 {menu_slot:#x}",
        scenes.scene_ids(),
        scenes.short_system_menu_open,
        scenes.pause_menu_open(),
        signals.ui_dialog_state,
    ));
}

pub(super) fn install(build: VerifiedBuild) {
    let base = build.module_base();
    MODULE_BASE.store(base, Ordering::Relaxed);
    #[expect(unsafe_code, reason = "hooks the control mask getter by address")]
    // SAFETY: the RVA starts the game-state control mask getter (game state pointer in, mask out)
    // in the verified build, and the replacement has the same signature.
    unsafe {
        hook_address(
            "GameState_GetControlMask",
            (base + u64::from(GET_CONTROL_MASK_FUNCTION_RVA)) as usize,
            get_control_mask_detour as *const () as *mut c_void,
            &ORIGINAL_GET_CONTROL_MASK,
        );
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original getter through the trampoline MinHook built"
)]
extern "system" fn get_control_mask_detour(game_state: *mut c_void) -> u32 {
    let original = ORIGINAL_GET_CONTROL_MASK.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    // SAFETY: the trampoline continues into the original getter, which has this exact signature.
    let mask = unsafe { core::mem::transmute::<*mut c_void, GetControlMask>(original)(game_state) };
    panic::catch_unwind(|| settle(mask)).unwrap_or(mask)
}

#[expect(
    unsafe_code,
    reason = "reads the event byte and the four mask slots in the game's image"
)]
fn settle(mask: u32) -> u32 {
    let base = MODULE_BASE.load(Ordering::Relaxed);
    let partner_connected = PARTNER_CONNECTED.load(Ordering::Relaxed);
    let menu_open = PAUSE_MENU_OPEN.load(Ordering::Relaxed);
    if base == 0 || (!APPLYING.load(Ordering::Relaxed) && !(partner_connected && menu_open)) {
        return mask;
    }
    // SAFETY: the event byte and the four mask dwords are fixed, aligned data in the verified
    // build's image (the dwords are the ones the original getter ORs); plain reads cannot fault.
    let (event_scene, slots) = unsafe {
        let first = (base + u64::from(CONTROL_MASK_FIRST_RVA)) as *const u32;
        let mut slots = [0_u32; CONTROL_MASK_SLOT_COUNT as usize];
        for (index, slot) in slots.iter_mut().enumerate() {
            *slot = ptr::read_volatile(first.add(index));
        }
        (
            ptr::read_volatile((base + u64::from(EVENT_SCENE_BYTE_RVA)) as *const u8),
            slots,
        )
    };
    let view = PauseView {
        partner_connected,
        menu_open,
        event_scene: event_scene != 0,
    };
    let applies = view.applies();
    let settled = if applies {
        effective_control_mask(slots, view)
    } else {
        mask
    };
    if APPLYING.swap(applies, Ordering::Relaxed) != applies {
        log_line(&if applies {
            format!(
                "co-op no pause: starts, the world keeps running under the menu (mask {mask:#x} -> {settled:#x})"
            )
        } else {
            format!(
                "co-op no pause: ends (mask {mask:#x}, partner connected {partner_connected}, pause menu open {menu_open}, event scene {})",
                view.event_scene
            )
        });
    }
    settled
}
