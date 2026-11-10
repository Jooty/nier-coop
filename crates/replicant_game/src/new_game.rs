mod first_save;
mod save_write;
mod title_hold;
mod title_step;

pub use first_save::{FirstSaveGate, FirstSaveInputs, FirstSaveStep, first_save_permitted};
pub use save_write::{SaveWriteProgress, save_write_progress};
pub use title_hold::{TitleHold, TitleHoldInputs, title_hold};
pub use title_step::{TitleStep, title_step};

pub const MENU_RESULT_NEW_GAME: i32 = 4;
pub const MENU_RESULT_NAME_CONFIRMED: i32 = 6;
pub const FIRST_SAVE_PHASE: &str = "A_TOKYO_FIELD_01";
pub const FIRST_SAVE_SETTLE_MS: u64 = 3_000;
pub const FIRST_SAVE_GIVE_UP_MS: u64 = 600_000;
pub const TITLE_HOLD_TIMEOUT_MS: u64 = 120_000;
pub const SAVE_WRITE_BUSY_GRACE_MS: u64 = 2_000;
pub const SAVE_WRITE_TIMEOUT_MS: u64 = 30_000;
