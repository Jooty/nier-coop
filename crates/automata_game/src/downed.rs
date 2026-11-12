mod downed_setting;
mod effect_text;
mod hero_guard;
mod item_hold;
mod pose;
mod prompt;
mod rules;
mod signal;

pub use downed_setting::downed_revive_from_text;
pub use effect_text::effect_log_line;
pub use hero_guard::{
    HeroGuard, HeroHit, HeroHitTreatment, hero_hit_treatment, hit_put_the_hero_down,
};
pub use item_hold::{
    AutoItemUseHold, AutoItemUseReading, ItemUseAsker, item_use_asker, item_use_is_held,
    own_item_use_is_held,
};
pub use pose::{
    PoseHandlerReading, downed_pose_start, handler_called_by_the_state_case,
    pose_handler_is_the_noted_one, pose_is_started_again, start_is_refused_in_the_downed_pose,
};
pub use prompt::{DownedPrompt, DownedView, downed_prompt, prompt_text};
pub use rules::{
    DownedEffect, DownedRules, FallBackCause, GotUpCause, StepInput, distance_between,
    health_after_getting_up, hero_is_defeated_by_the_game, revived_health,
};
pub use signal::{DownedPhase, DownedSignal};

pub const REVIVE_RANGE_UNITS: f32 = 3.0;
pub const REVIVE_HOLD_SECONDS: f32 = 3.0;
pub const DOWNED_HEALTH: i32 = 1;
pub const BLEED_OUT_SECONDS: f32 = 60.0;
pub const REVIVED_HEALTH_DIVISOR: i32 = 4;
pub const MIN_REVIVED_HEALTH: i32 = 2;
pub const REVIVED_SHIELD_SECONDS: f32 = 3.0;
pub const REVIVE_PERCENT_WHEN_DONE: u8 = 100;
pub const HIGHEST_REVIVE_PERCENT_BEFORE_DONE: u8 = 99;
pub const REVIVE_PROGRESS_LOG_STEP_PERCENT: u8 = 25;
pub const LONGEST_STEP_SECONDS: f32 = 0.25;
pub const DOWNED_POSE_STATE: i32 = 0x2D;
pub const DOWNED_POSE_START_INTERVAL_MS: u64 = 1_000;
pub const FALL_BACK_RETRY_INTERVAL_MS: u64 = 2_000;
pub const FALL_BACK_TRIES_AT_MOST: u32 = 5;
pub const HERO_DEFEAT_START_STATE: u16 = 0x35;
pub const DOWNED_LINES_LOGGED_AT_MOST: u32 = 400;
pub const BODY_FLIGHT_FLAG_OFFSETS: [u32; 2] = [0x1_5B68, 0x1_6A70];
pub const BODY_OWN_DEFEAT_FLOOR_SWITCH_OFFSET: u32 = 0x1_6FD8;
pub const BODY_OWN_DEFEAT_FLOOR_MODE_OFFSET: u32 = 0x1_74AC;
pub const OWN_DEFEAT_FLOOR_MODE_THAT_FLOORS: i32 = 1;
pub const STATE_DISPATCH_TABLE_ENTRY_BYTES: u32 = 4;
pub const STATE_CASE_BYTES_READ: usize = 8;
pub const STATE_CASE_CALL_START: [u8; 4] = [0x48, 0x8B, 0xCF, 0xE8];
pub const BLOWN_DOWN_STATE_HANDLER_START: [u8; 24] = [
    0x40, 0x53, 0x48, 0x83, 0xEC, 0x70, 0x48, 0x63, 0x81, 0x74, 0x06, 0x00, 0x00, 0x48, 0x8B, 0xD9,
    0x83, 0xF8, 0x07, 0x0F, 0x87, 0x2A, 0x04, 0x00,
];
pub const DOWNED_REVIVE_SETTING_ON_TEXT: &str = "on";
pub const DOWNED_REVIVE_SETTING_OFF_TEXT: &str = "off";
pub const ITEM_IDS_THAT_HEAL_THE_HERO: [i32; 5] = [0, 1, 2, 3, 0x5A];
pub const AUTO_ITEM_USE_CHIP_TYPE: i32 = 0x1B;
pub const AUTO_ITEM_USE_HEALTH_SHARE: f32 = 0.3;
pub const AUTO_ITEM_USE_COOL_DOWN_WHILE_HEALTH_IS_HIGH: f32 = 48.0;
pub const BODY_AUTO_ITEM_USE_COOL_DOWN_OFFSET: u32 = 0x1_78E4;
pub const BODY_STEP_OFFSET: u32 = 0x1470;
pub const ITEM_USES_HELD_LOGGED_AT_MOST: u32 = 80;
pub const USE_ITEM_FUNCTION_START: [u8; 16] = [
    0x40, 0x53, 0x55, 0x57, 0x48, 0x83, 0xEC, 0x60, 0x0F, 0x29, 0x74, 0x24, 0x50, 0x48, 0x8B, 0x05,
];
pub const AUTO_ITEM_USE_CHIP_CHECK_FUNCTION_START: [u8; 12] = [
    0x40, 0x53, 0x48, 0x81, 0xEC, 0xA0, 0x01, 0x00, 0x00, 0x48, 0x8B, 0x05,
];
pub const CHIP_LEVEL_FUNCTION_START: [u8; 18] = [
    0x44, 0x8B, 0xC2, 0x4C, 0x8B, 0xC9, 0x83, 0xFA, 0x1D, 0x74, 0x05, 0x83, 0xFA, 0x24, 0x75, 0x0F,
    0xF6, 0x05,
];
