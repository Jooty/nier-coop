mod ask_answer;
mod ask_verdict;
mod follow_lines;
mod follow_machine;
mod follow_step;
mod game_request_verdict;
mod guest_side;
mod host_side;
mod request_class;
mod request_readiness;
mod request_to_make;
mod story_ask;
mod story_move;
mod story_request;

pub use ask_answer::StoryAskAnswer;
pub use ask_verdict::AskVerdict;
pub use follow_lines::{
    answer_heard_line, ask_kept_waiting_line, ask_sent_line, catch_up_line, gave_up_line,
    host_ask_answered_line, host_ask_made_line, in_flight_expired_line, link_lost_line,
    made_here_line, move_dropped_as_already_here_line, move_heard_line, move_told_line,
    request_kept_line, request_not_told_line, request_replaced_line,
};
pub use follow_machine::{FollowMachine, FollowReadings};
pub use follow_step::{FollowSend, FollowStep};
pub use game_request_verdict::GameRequestVerdict;
pub use request_class::RequestClass;
pub use request_readiness::RequestReadiness;
pub use story_ask::StoryAsk;
pub use story_move::StoryMove;
pub use story_request::StoryRequest;

pub const HOST_KEEPS_AN_ASK_WHILE_BUSY_MS: u64 = 10_000;
pub const STORY_ASK_RESEND_MS: u64 = 1_000;
pub const STORY_ASK_GIVE_UP_MS: u64 = 20_000;
pub const STORY_PLACE_CATCH_UP_AFTER_MS: u64 = 5_000;
pub const REQUEST_IN_FLIGHT_MOST_MS: u64 = 15_000;
pub const RELEASED_REQUEST_TRIED_MOST_MS: u64 = 20_000;
pub const MOST_MOVES_QUEUED_AT_THE_GUEST: usize = 16;
pub const MOST_ASKS_KEPT_AT_THE_HOST: usize = 8;
pub const MOST_ANSWERED_ASKS_REMEMBERED: usize = 16;
pub const DEFAULT_FULL_REQUEST_PARAMS: [i32; 6] = [0, 0, 0x78, 0, 0, 0];
pub const PHASE_JUMP_FLAG_WORD_MASK_AFTER_A_SUB_PHASE_REQUEST: u32 = 0xFF01_3FFF;
pub const PHASE_SYSTEM_IDLE_STATE: i32 = 2;
pub const NO_REQUESTED_PHASE_ID: i32 = -1;
