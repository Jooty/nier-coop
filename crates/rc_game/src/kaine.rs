mod partner_hide;
mod save_guard;

pub use partner_hide::{PartnerHideStep, PartnerKaineHide, partner_kaine_hidden_wanted};
pub use save_guard::{KaineCopyPutBack, put_back_kaine_copies};

use crate::offsets::SAVE_FORCED_WEAPON_FLAG_OFFSET;

pub const KAINE_PLAYER_FLAG_SAVE_OFFSET: usize = SAVE_FORCED_WEAPON_FLAG_OFFSET as usize;
pub const KAINE_COPY_SAVE_START: usize = KAINE_PLAYER_FLAG_SAVE_OFFSET;
pub const KAINE_COPY_SAVE_END: usize = 0x1161;
pub const SEED_INDEX_SAVE_OFFSET: usize = 0x28;
pub const KAINE_PLAYER_FLAG_SET: i32 = 1;
pub const PARTNER_EXECUTES_FLAG_OFFSET: u64 = 0x13E4;
