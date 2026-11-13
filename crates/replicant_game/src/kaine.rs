mod hit_power;
mod partner_hide;
mod save_guard;
mod stats;

pub use hit_power::{
    HeldWeapon, PERCENT_PER_WHOLE, SWAPPED_POWER_LINE_INTERVAL_MS, SwappedBodyPower,
    SwappedHitTally, SwappedPowerLog, WeaponPercents, read_minimum_damages, swapped_power_line,
    weapon_factor, weapon_powered_source,
};
pub use partner_hide::{PartnerHideStep, PartnerKaineHide, partner_kaine_hidden_wanted};
pub use save_guard::{KaineCopyPutBack, put_back_kaine_copies};
pub use stats::{
    KaineStatsSynced, clamped_current, level_row_in_range, param_patches, sync_kaine_stats,
};

use crate::offsets::SAVE_FORCED_WEAPON_FLAG_OFFSET;

pub const KAINE_PLAYER_FLAG_SAVE_OFFSET: usize = SAVE_FORCED_WEAPON_FLAG_OFFSET as usize;
pub const KAINE_COPY_SAVE_START: usize = KAINE_PLAYER_FLAG_SAVE_OFFSET;
pub const KAINE_COPY_SAVE_END: usize = 0x1161;
pub const SEED_INDEX_SAVE_OFFSET: usize = 0x28;
pub const KAINE_PLAYER_FLAG_SET: i32 = 1;
pub const PARTNER_EXECUTES_FLAG_OFFSET: u64 = 0x13E4;
pub const KAINE_LEVEL_SAVE_OFFSET: usize = 0xE3C;
pub const LEVEL_STATS_TABLE_RVA: u32 = 0x47C_2638;
pub const LEVEL_STATS_ROW_BYTES: usize = 0x2C;
pub const MAX_LEVEL_STATS_ROW: i32 = 0x62;
pub const STATS_ROW_TO_PARAM: [(usize, u64); 9] = [
    (0x04, 0x18),
    (0x08, 0x1C),
    (0x0C, 0x3C),
    (0x10, 0x40),
    (0x14, 0x44),
    (0x18, 0x48),
    (0x1C, 0x30),
    (0x20, 0x4C),
    (0x24, 0x50),
];
