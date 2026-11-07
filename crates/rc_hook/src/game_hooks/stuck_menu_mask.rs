use core::ffi::c_void;
use std::sync::{Mutex, PoisonError};

use rc_game::GameMemory;
use rc_game::coop_pause::{LEFTOVER_PAUSE_MENU_MASK, MaskView, StuckMenuMask};
use rc_game::dialogue::{CONTROL_MASK_SET_SLOT_ZERO_RVA, GAME_STATE_RVA};
use rc_game::marker::{CONTROL_MASK_FIRST_RVA, EVENT_SCENE_BYTE_RVA};
use rc_game::ui_scenes::OpenUiScenes;

use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::net_clock_ms;

const HUD_UI_SCENE_ID: u8 = 1;

static WATCH: Mutex<StuckMenuMask> = Mutex::new(StuckMenuMask::new());

type SetMaskSlot = unsafe extern "system" fn(*mut c_void, u32);

pub(super) fn watch(memory: &InProcessMemory, scenes: OpenUiScenes, ui_dialog_state: i32) {
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
    };
    let stuck_ms = WATCH
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .observe(view, net_clock_ms());
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
