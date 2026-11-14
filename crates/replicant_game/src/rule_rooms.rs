mod log_text;
mod lookup;
mod room;
mod table;
mod watch;

pub use log_text::{armed_line, guest_break_line, kept_message_line};
pub use lookup::{RuleRoomCall, is_host_run_rule_room_call, rule_room_call};
pub use room::{RoomInhibit, RuleAct, RuleBreak, RuleRoom};
pub use table::RULE_ROOMS;
pub use watch::{GuestBreakVerdict, RulePlayer, RulePop, RulePopFacts, RuleRoomWatch, RuleState};

pub const PLAYER_DASHED_MESSAGE: u32 = 216;
pub const PLAYER_JUMPED_MESSAGE: u32 = 219;
pub const PLAYER_CAST_MAGIC_MESSAGES: [u32; 8] = [220, 221, 222, 223, 224, 225, 226, 227];
pub const PLAYER_GUARDED_MESSAGE: u32 = 228;
pub const PLAYER_ATTACKED_MESSAGE: u32 = 229;
pub const PLAYER_HIT_BY_THE_ROOM_MESSAGE: u32 = 422;
