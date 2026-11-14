mod lead_follow;
mod lead_messages;
mod place_log;
mod place_rule;
mod start_tally;
mod target_rule;

pub use lead_follow::LeadFollower;
pub use lead_messages::{EnemyLead, EnemyPose, LeadTarget, lead_parts};
pub use place_log::{PlaceLog, place_line};
pub use place_rule::{PlaceStep, facing_to_write, place_step};
pub use start_tally::{HookedStarts, StartTally};
pub use target_rule::{
    PlayerPlaces, guest_target_handle, lead_target_of, target_wanted_on_the_host,
};

pub const ENEMY_POSITION_OFFSET: u32 = 0x50;
pub const ENEMY_FACING_OFFSET: u32 = 0x94;
pub const ENEMY_STATE_OFFSET: u32 = 0x670;
pub const ENEMY_TARGET_HANDLE_OFFSET: u32 = 0x5EB4;
pub const ENEMY_STATE_START_FUNCTION_START: [u8; 32] = [
    0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18, 0x57,
    0x48, 0x83, 0xEC, 0x30, 0x83, 0xB9, 0x00, 0xC1, 0x02, 0x00, 0x00, 0x41, 0x8B, 0xF1, 0x41, 0x8B,
];
pub const ENEMY_TARGET_SETTER_FUNCTION_START: [u8; 32] = [
    0x40, 0x57, 0x48, 0x83, 0xEC, 0x40, 0x48, 0xC7, 0x44, 0x24, 0x20, 0xFE, 0xFF, 0xFF, 0xFF, 0x48,
    0x89, 0x5C, 0x24, 0x50, 0x48, 0x8B, 0xFA, 0x48, 0x8B, 0xD9, 0x83, 0xB9, 0xB0, 0x5E, 0x00, 0x00,
];

pub const ENEMY_LEAD_IS_SENT_EVERY_MS: u64 = 100;
pub const ENEMY_POSES_IN_ONE_LEAD_MESSAGE: usize = 8;
pub const MOST_ENEMIES_LED: usize = 128;
pub const MOST_ENEMY_LEADS_WAITING: usize = 48;
pub const HOLD_LIFTS_AFTER_THE_HOSTS_SILENCE_MS: u64 = 1_000;
pub const ENEMY_IS_HELD_ONCE_IT_HAS_LIVED_HERE_MS: u64 = 1_500;
pub const ENEMY_PLACE_IS_LOGGED_EVERY_MS: u64 = 2_000;
pub const PLACE_LINES_LOGGED_PER_MINUTE: u32 = 20;
pub const PLACE_ERROR_LEFT_ALONE: f32 = 0.05;
pub const PLACE_ERROR_SNAPPED: f32 = 6.0;
pub const PLACE_NUDGE_FRACTION_PER_FRAME: f32 = 0.25;
pub const FACING_ERROR_LEFT_ALONE_RADIANS: f32 = 0.05;
pub const FACING_NUDGE_FRACTION_PER_FRAME: f32 = 0.35;
pub const TARGET_SWITCHES_WHEN_NEARER_BY: f32 = 3.0;
pub const HIGHEST_POSE_COORDINATE_ABS: f32 = 1_000_000.0;
pub const HIGHEST_POSE_FACING_ABS_RADIANS: f32 = 16.0;
pub const HOST_STARTS_LEFT_TO_THE_DEFEAT_PATH: [i32; 3] = [0x8E, 0xAC, 0xAF];
