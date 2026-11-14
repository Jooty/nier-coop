mod breaker_return;
mod log_text;
mod lookup;
mod rearm_after_restart;
mod return_spot;
mod room;
mod room_clock;
mod table;
mod take_in;
mod watch;

pub use breaker_return::BreakerReturn;
pub(crate) use log_text::room_name;
pub use log_text::{
    armed_line, guest_break_line, kept_message_line, returned_line, returned_without_a_spot_line,
};
pub use lookup::{RuleRoomCall, is_host_run_rule_room_call, rule_room_call};
pub use rearm_after_restart::RearmAfterRestart;
pub use return_spot::{RETURN_SPOTS, ReturnSpot, return_spot_of};
pub use room::{RoomInhibit, RuleAct, RuleBreak, RuleRoom};
pub use room_clock::RoomClock;
pub use table::RULE_ROOMS;
pub use take_in::{TakeIn, take_in_failed_line};
pub use watch::{RoomResetVerdict, RulePlayer, RulePop, RulePopFacts, RuleRoomWatch, RuleState};

pub const GUEST_ALREADY_BESIDE_THE_HOST_UNITS: f32 = 3.0;
pub const PLAYER_DASHED_MESSAGE: u32 = 216;
pub const PLAYER_JUMPED_MESSAGE: u32 = 219;
pub const PLAYER_CAST_MAGIC_MESSAGES: [u32; 8] = [220, 221, 222, 223, 224, 225, 226, 227];
pub const PLAYER_GUARDED_MESSAGE: u32 = 228;
pub const PLAYER_ATTACKED_MESSAGE: u32 = 229;
pub const PLAYER_HIT_BY_THE_ROOM_MESSAGE: u32 = 422;
pub const RETURN_FADE_OUT_MS: u64 = 700;
pub const RETURN_FADE_OUT_SECONDS: f32 = 0.7;
pub const RETURN_FADE_IN_SECONDS: f32 = 1.5;
pub const BLACK_FADE_KIND: i32 = 2;
pub const BLACK_FADE_DEPTH: u32 = 0x20;
pub const RETURN_INVINCIBLE_SECONDS: f64 = 1.8;
pub const CHANGE_CAMERA_BACK_TO_THE_PLAYER_SECONDS: f64 = 0.0;
pub const CHANGE_CAMERA_BLEND_KIND: i32 = 1;
pub const REMOVE_CALLBACK_FROM_ID_NATIVE_RVA: u32 = 0x6F_1660;
pub const SET_PLAYER_VISIBLE_NATIVE_RVA: u32 = 0x70_08A0;
pub const SET_UNIQUE_ACTOR_INVINCIBLE_NATIVE_RVA: u32 = 0x70_2DD0;
pub const RESET_FREE_CAMERA_NATIVE_RVA: u32 = 0x6F_20B0;
pub const SET_SNOW_ACTIVE_AREA_BY_PLAYER_POSITION_NATIVE_RVA: u32 = 0x70_1D80;
pub const ROOM_CLOCK_SECONDS: u64 = 120;
pub const ROOM_CLOCK_LOG_EVERY_MS: u64 = 10_000;
pub const REARM_NOT_BEFORE_RESTART_POP_MS: u64 = 1_500;
