mod defeat_note;
mod defeat_way;
mod game_over_menu;
mod menu_write;
mod partner_body_guard;
mod restore_place;
mod shared_defeat_lines;
mod shared_defeat_machine;
mod shared_defeat_readings;
mod shared_defeat_step;

pub use defeat_note::DefeatNote;
pub use defeat_way::DefeatWay;
pub use game_over_menu::{GameOverChoice, GameOverMenu};
pub use menu_write::GameOverMenuWrite;
pub use partner_body_guard::{PartnerBodyGuardAct, PartnerBodyReading, partner_body_guard_act};
pub use restore_place::RestorePlace;
pub use shared_defeat_lines::{
    follow_line, heard_choice_after_own_line, heard_choice_dropped_for_the_way_line,
    heard_choice_gave_up_line, heard_choice_line, heard_choice_waits_line, local_flow_over_line,
    local_way_line, menu_write_done_line, no_partner_any_more_line, own_choice_first_line,
    own_choice_line, partner_body_guard_line, partner_way_line, partner_way_text, restore_text,
    retry_not_followed_line, shared_defeat_off_line, shared_defeat_on_line, told_restore_text,
    ways_differ_line,
};
pub use shared_defeat_machine::SharedDefeatMachine;
pub use shared_defeat_readings::SharedDefeatReadings;
pub use shared_defeat_step::SharedDefeatStep;

pub const GAME_OVER_MENU_STATE_OFFSET: u64 = 0x8;
pub const GAME_OVER_MENU_SUB_STATE_OFFSET: u64 = 0xC;
pub const GAME_OVER_MENU_CURSOR_OFFSET: u64 = 0x110;
pub const GAME_OVER_MENU_CHOOSING_STATE: i32 = 3;
pub const GAME_OVER_MENU_SECOND_LIST_STATE: i32 = 4;
pub const GAME_OVER_MENU_FADE_OUT_STATE: i32 = 5;
pub const GAME_OVER_MENU_RETRY_CHOSEN_STATE: i32 = 6;
pub const GAME_OVER_MENU_CHOOSING_SUB_STATE: i32 = 2;
pub const GAME_OVER_MENU_TITLE_CURSOR: i32 = 1;
pub const GAME_OVER_MENU_RETRY_CURSOR: i32 = 0;
pub const IN_PLACE_RETRY_BIT: u32 = 0x1000_0000;
pub const RETRY_PHASE_CHANGE_PARAMS: [i32; 6] = [1, 0, 0x78, 0, 0, 0];
pub const RESTORE_POINT_PLACE_RECORD_OFFSET: u64 = 0x3_95E0;
pub const PLACE_RECORD_PHASE_ID_OFFSET: u64 = 0x4;
pub const PLACE_RECORD_SUB_PHASE_NAME_OFFSET: u64 = 0x8;
pub const PLACE_RECORD_HAS_A_POINT_FLAG: u32 = 0x2000_0000;
pub const HEARD_DEFEAT_NOTE_KEPT_FOR_MS: u64 = 30_000;
pub const SHARED_DEFEAT_WAIT_LINE_INTERVAL_MS: u64 = 1_000;
pub const SHARED_DEFEAT_LINES_LOGGED_AT_MOST: u32 = 2_000;
pub const BODY_FLIGHT_DEFEAT_STATE: i32 = 0x91;
pub const BODY_STATE_THAT_ASKS_FOR_A_DEFEAT_AT_NO_HEALTH: i32 = 0xC5;
pub const BODY_FLIGHT_DEFEAT_TIMER_OFFSET: u32 = 0x1A74;
pub const FLIGHT_DEFEAT_TIMER_THAT_NEVER_ASKS: f32 = 0.0;
pub const PARTNER_BODY_GUARD_LINES_LOGGED_AT_MOST: u32 = 40;
