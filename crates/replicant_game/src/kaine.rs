mod companion_off;
mod companion_off_fights;
mod hit_power;
mod level_in_use;
mod partner_hide;
mod save_guard;
mod stand_in;
mod stand_in_beats;
mod stats;
mod swapped_level;

pub use companion_off::{CompanionOff, CompanionOffEvent, CompanionOffReport, CompanionOffStep};
pub use companion_off_fights::{
    COMPANION_OFF_FIGHTS, CompanionOffFight, companion_off_fight_started_by,
};

pub use hit_power::{
    HeldWeapon, PERCENT_PER_WHOLE, SWAPPED_POWER_LINE_INTERVAL_MS, SwappedBodyPower,
    SwappedHitTally, SwappedPowerLog, WeaponPercents, read_minimum_damages, swapped_power_line,
    weapon_factor, weapon_powered_source,
};
pub use level_in_use::{LevelFields, level_fields_in_use};
pub use partner_hide::{PartnerHideStep, PartnerKaineHide, partner_kaine_hidden_wanted};
pub use save_guard::{KaineCopyPutBack, put_back_kaine_copies};
pub use stand_in::{
    GuestStandIn, HeldTrap, OwnTrapVerdict, StandInEvent, StandInFacts, StandInReport, StandInSide,
};
pub use stand_in_beats::{
    STAND_IN_BEATS, StandInBeat, open_stand_in_beat, stand_in_beat_moved_after, stand_in_hp_damage,
};
pub use stats::{
    KaineStatsSynced, clamped_current, level_row_in_range, param_patches, sync_kaine_stats,
};
pub use swapped_level::{
    SwappedBodyLevel, SwappedLevelChange, SwappedLevelStep, swapped_level_line,
};

use crate::offsets::SAVE_FORCED_WEAPON_FLAG_OFFSET;

pub const KAINE_PLAYER_FLAG_SAVE_OFFSET: usize = SAVE_FORCED_WEAPON_FLAG_OFFSET as usize;
pub const KAINE_COPY_SAVE_START: usize = KAINE_PLAYER_FLAG_SAVE_OFFSET;
pub const KAINE_COPY_SAVE_END: usize = 0x1161;
pub const SEED_INDEX_SAVE_OFFSET: usize = 0x28;
pub const KAINE_PLAYER_FLAG_SET: i32 = 1;
pub const PARTNER_EXECUTES_FLAG_OFFSET: u64 = 0x13E4;
pub const KAINE_LEVEL_SAVE_OFFSET: usize = 0xE3C;
pub const KAINE_EXPERIENCE_SAVE_OFFSET: usize = 0xE40;
pub const COMPANION_GAUGE_MODE_OFFSET: u64 = 0x1_5E94;
pub const HUD_PARTNER_GAUGE_OFFSET: u64 = 0x288;
pub const PARTNER_GAUGE_ACTOR_HANDLE_OFFSET: u64 = 0x70;
pub const GUEST_HIT_SCALE_IN_A_STAND_IN_BEAT: f32 = 0.25;
pub const HELD_TRAP_RELEASE_DELAY_MS: u64 = 3_000;
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
