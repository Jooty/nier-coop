mod defeat_flow;
mod flow_log;
mod health_log;
mod hold_log;
mod left_body_record;
mod partner_body_harm;

pub use defeat_flow::{DefeatFlow, is_hero_defeat_state};
pub use flow_log::{DefeatFlowLog, RestorePoint};
pub use health_log::HealthLog;
pub use hold_log::{HoldLog, OtherPlayerHold, hold_cause};
pub use left_body_record::left_body_record_is_dropped;
pub use partner_body_harm::{
    health_kept_on_the_partner_body, local_game_may_start_on_the_held_body,
};

pub const BODY_MAX_HEALTH_BASE_OFFSET: u32 = 0x85C;
pub const BODY_MAX_HEALTH_BONUS_OFFSET: u32 = 0x1_6948;
pub const HERO_DEFEAT_STATES: [i32; 2] = [0x35, 0x36];
pub const DEFEAT_FLOW_IDLE_STATE: i32 = 0;
pub const DEFEAT_FLOW_REQUESTED_STATE: i32 = 1;
pub const DEFEAT_FLOW_CHOOSING_STATE: i32 = 2;
pub const DEFEAT_FLOW_USUAL_STATE: i32 = 3;
pub const DEFEAT_FLOW_TITLE_STATE: i32 = 7;
pub const DEFEAT_FLOW_ENDING_STATE: i32 = 8;
pub const DEFEAT_FLOW_FADE_STEP: i32 = 0;
pub const DEFEAT_FLOW_LEFT_BODY_SCREEN_STEPS: [i32; 2] = [1, 2];
pub const DEFEAT_FLOW_CONTINUE_STEP: i32 = 10;
pub const DEFEAT_FLOW_WAIT_FOR_THE_CONTINUE_STEP: i32 = 11;
pub const RESTORE_POINT_MARK: u16 = 0x5954;
pub const HEALTH_CHANGE_WORTH_A_LINE_PERCENT: i32 = 10;
pub const HEALTH_LINES_LOGGED_PER_MINUTE: u32 = 60;
pub const DEFEAT_FLOW_LINES_LOGGED_PER_MINUTE: u32 = 60;
pub const DEFEAT_FLOW_STALL_LINE_INTERVAL_MS: u64 = 10_000;
pub const HOLD_LINES_LOGGED_PER_MINUTE: u32 = 60;
pub const LOWEST_HEALTH_KEPT_ON_THE_PARTNER_BODY: i32 = 1;
pub const NO_LEFT_BODY_RECORD: i32 = 0;
pub const PLAYER_DAMAGE_VTABLE_OFFSET: u32 = 0x4B8;
pub const PLAYER_DAMAGE_FUNCTION_START: [u8; 28] = [
    0x48, 0x8B, 0xC4, 0x57, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x41, 0x57, 0x48, 0x81, 0xEC, 0xA0,
    0x00, 0x00, 0x00, 0x48, 0xC7, 0x44, 0x24, 0x38, 0xFE, 0xFF, 0xFF, 0xFF,
];
pub const DEFEAT_FLOW_CONTINUE_WITHOUT_LEFT_BODY_STATE: i32 = 4;
pub const DEFEAT_FLOW_GAME_OVER_MENU_STATE: i32 = 5;
pub const DEFEAT_FLOW_FIXED_PHASE_REQUEST_STATE: i32 = 6;
pub const DEFEAT_FLOW_LAST_STEP_BEFORE_THE_END: i32 = 12;
pub const DEFEAT_FLOW_MENU_DROPS_THE_RECORD_STEP: i32 = 0;
pub const DEFEAT_FLOW_MENU_SETS_ITS_COUNTER_STEP: i32 = 1;
pub const DEFEAT_FLOW_MENU_FADES_IN_STEP: i32 = 2;
pub const DEFEAT_FLOW_MENU_OPENS_STEP: i32 = 3;
pub const DEFEAT_FLOW_MENU_WAITS_FOR_A_CHOICE_STEP: i32 = 4;
pub const DEFEAT_FLOW_MENU_CLOSES_STEP: i32 = 5;
pub const CURRENT_POINT_STATE_WORD_THE_CONTINUE_WAITS_FOR: u32 = 0xDFA8_21B1;
