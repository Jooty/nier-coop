mod menu_taint;
mod radio_sites;
mod rule;
mod stuck_menu_mask;

pub use menu_taint::{MenuTaint, TalkWindowTaints};
pub use radio_sites::{
    FIND_SCENE_ROOT_FUNCTION_RVA, GET_SLOT_ZERO_FUNCTION_RVA, NEAR_CALL_LENGTH,
    PAUSE_MENU_SCENE_KIND, RADIO_CLOCK_SCENE_QUERY_CALL_SITE_RVAS, RADIO_SLOT_ZERO_CALL_SITE_RVAS,
    SET_SLOT_ZERO_FUNCTION_RVA, TALK_WINDOW_CLOSE_SLOT_ZERO_RESTORE_CALL_SITE_RVA,
    TALK_WINDOW_OPEN_SLOT_ZERO_SAVE_CALL_SITE_RVA, menu_added_bits, near_call_bytes,
    radio_clock_scene_query_hidden, radio_slot_zero, talk_window_restored_slot_zero,
};
pub use rule::{PauseView, effective_control_mask, mask_without_menu, ui_prompt_open};
pub use stuck_menu_mask::{MaskView, StuckMenuMask, StuckOtherSlot};

pub const MENU_WORLD_FREEZE_BITS: u32 = 0x3F;
pub const MENU_OWN_BITS: u32 = 0x1BF;
pub const MENU_PLAYER_INPUT_BIT: u32 = 0x80;
pub const MENU_RADIO_PAUSE_BIT: u32 = 0x100;
pub const LEFTOVER_PAUSE_MENU_MASK: u32 = 0x19F;
pub const LEFTOVER_PAUSE_MENU_MASK_GRACE_MS: u64 = 2_000;
