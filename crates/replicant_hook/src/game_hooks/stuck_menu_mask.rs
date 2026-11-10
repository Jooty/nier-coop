use core::ffi::c_void;
use std::sync::{Mutex, PoisonError};

use replicant_game::GameMemory;
use replicant_game::coop_pause::{LEFTOVER_PAUSE_MENU_MASK, MaskView, StuckMenuMask};
use replicant_game::dialogue::{CONTROL_MASK_SET_SLOT_ZERO_RVA, GAME_STATE_RVA, TALK_WINDOW_RVA};
use replicant_game::marker::{CONTROL_MASK_FIRST_RVA, EVENT_SCENE_BYTE_RVA};
use replicant_game::ui_scenes::OpenUiScenes;

use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::net_clock_ms;

const HUD_UI_SCENE_ID: u8 = 1;

static WATCH: Mutex<StuckMenuMask> = Mutex::new(StuckMenuMask::new());

type SetMaskSlot = unsafe extern "system" fn(*mut c_void, u32);

pub(super) fn watch(
    memory: &InProcessMemory,
    scenes: OpenUiScenes,
    ui_dialog_state: i32,
    in_play: bool,
) {
    let base = memory.module_base();
    let mut slots = [0_u32; 4];
    for (index, slot) in slots.iter_mut().enumerate() {
        *slot = memory
            .read_i32(base + u64::from(CONTROL_MASK_FIRST_RVA) + 4 * index as u64)
            .unwrap_or(0)
            .cast_unsigned();
    }
    let view = MaskView {
        slots,
        extra_ui_scene_open: scenes.pause_menu_open()
            || scenes.scene_ids().iter().any(|id| *id != HUD_UI_SCENE_ID),
        ui_dialog_state,
        event_scene: memory
            .read_u8(base + u64::from(EVENT_SCENE_BYTE_RVA))
            .is_ok_and(|byte| byte != 0),
        talk_window_open: memory
            .read_i32(base + u64::from(TALK_WINDOW_RVA))
            .is_ok_and(|state| state != 0),
        in_play,
    };
    let now = net_clock_ms();
    let mut watch = WATCH.lock().unwrap_or_else(PoisonError::into_inner);
    let stuck_ms = watch.observe(view, now);
    let stuck_other = watch.observe_other_slots(view, now);
    drop(watch);
    if let Some(stuck) = stuck_other {
        log_line(&format!(
            "control: released a leftover mask {:#x} in slot {} (no talk, menu or scene open)",
            stuck.mask, stuck.slot
        ));
        clear_slot(base, stuck.slot);
    }
    let Some(stuck_ms) = stuck_ms else {
        return;
    };
    log_line(&format!(
        "control recovery: the closed pause menu left mask {LEFTOVER_PAUSE_MENU_MASK:#x} in slot 0 for {stuck_ms} ms with no menu, dialog or scene open; slot 0 cleared, you can move again"
    ));
    clear_slot_zero(base);
}

#[expect(
    unsafe_code,
    reason = "calls the control mask slot 0 setter by address"
)]
fn clear_slot_zero(base: u64) {
    // SAFETY: the slot 0 setter (game state, value) in the verified build, called on the game
    // thread like the talk window mirror does.
    unsafe {
        core::mem::transmute::<usize, SetMaskSlot>(
            (base + u64::from(CONTROL_MASK_SET_SLOT_ZERO_RVA)) as usize,
        )((base + u64::from(GAME_STATE_RVA)) as *mut c_void, 0);
    }
}

#[expect(
    unsafe_code,
    reason = "writes one control mask dword in the game's static data"
)]
fn clear_slot(base: u64, slot: usize) {
    // SAFETY: the four control mask dwords are fixed, aligned data in the verified build's image
    // (menu_no_pause reads them the same way); slot is 1..=3, written on the game thread.
    unsafe {
        core::ptr::write_volatile(
            (base + u64::from(CONTROL_MASK_FIRST_RVA) + 4 * slot as u64) as *mut u32,
            0,
        );
    }
}
