mod rule;

pub use rule::{PauseView, effective_control_mask};

pub const MENU_WORLD_FREEZE_BITS: u32 = 0x3F;
pub const MENU_PLAYER_INPUT_BIT: u32 = 0x80;
