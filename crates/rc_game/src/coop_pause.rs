mod rule;
mod stuck_menu_mask;

pub use rule::{PauseView, effective_control_mask, mask_without_menu, ui_prompt_open};
pub use stuck_menu_mask::{MaskView, StuckMenuMask, StuckOtherSlot};

pub const MENU_WORLD_FREEZE_BITS: u32 = 0x3F;
pub const MENU_OWN_BITS: u32 = 0x1BF;
pub const MENU_PLAYER_INPUT_BIT: u32 = 0x80;
pub const LEFTOVER_PAUSE_MENU_MASK: u32 = 0x19F;
pub const LEFTOVER_PAUSE_MENU_MASK_GRACE_MS: u64 = 2_000;
