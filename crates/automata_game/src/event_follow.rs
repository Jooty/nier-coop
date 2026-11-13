mod event_ask;
mod event_call;
mod event_id;
mod event_lines;
mod event_machine;
mod event_readings;
mod event_slot;
mod event_step;
mod guest_side;
mod host_side;
mod partner_report;
mod play_verdict;
mod playing_here;
mod runs_for_the_partner;
mod seen_events;
mod skip_prompt;
mod skip_verdict;
mod skip_vote;
mod skip_vote_lines;
mod start_origin;
mod unattended;

pub use event_ask::{EventAsk, EventAskWhat};
pub use event_call::EventCall;
pub use event_id::EventId;
pub use event_lines::{
    ask_refused_line, ask_retold_line, ask_skip_ignored_line, ask_skip_made_line,
    ask_start_made_line, ended_here_line, follow_start_line, follow_waits_line, held_gave_up_line,
    held_released_line, held_waiting_line, host_end_heard_line, host_start_heard_line,
    link_lost_line, outlast_skip_line, outlast_started_line, play_held_line, play_passed_line,
    replay_skipped_line, run_gave_up_line, run_released_line, skip_ask_gave_up_line,
    skip_swallowed_line, skipped_for_the_host_line, started_here_line, told_line,
};
pub use event_machine::EventMachine;
pub use event_readings::EventReadings;
pub use event_slot::{EventSlot, slots_of_table};
pub use event_step::{EventSend, EventStep};
pub use partner_report::PartnerReport;
pub use play_verdict::PlayVerdict;
pub use playing_here::{PlayingHere, story_event_playing};
pub use runs_for_the_partner::RunsForThePartner;
pub use seen_events::SeenEvents;
pub use skip_prompt::SkipPrompt;
pub use skip_verdict::SkipVerdict;
pub use skip_vote::{SkipVote, VoteView};
pub use skip_vote_lines::{
    agreed_as_unattended_line, agreed_by_own_input_line, attendance_line, own_skip_ask_lapsed_line,
    own_skip_ask_line, partner_counted_as_agreeing_line, partner_does_not_count_line,
    partner_skip_ask_heard_line, partner_skip_ask_lapsed_line, vote_passed_line,
};
pub use start_origin::StartOrigin;
pub use unattended::Unattended;

pub const HOST_EVENT_START_REMEMBERED_MS: u64 = 20_000;
pub const EVENT_ASK_RESEND_MS: u64 = 1_000;
pub const GUEST_EVENT_START_GIVE_UP_MS: u64 = 15_000;
pub const EVENT_FOLLOW_GRACE_MS: u64 = 3_000;
pub const EVENT_SEEN_REMEMBERED_MS: u64 = 600_000;
pub const GUEST_EVENT_MAY_OUTLAST_THE_HOSTS_BY_MS: u64 = 5_000;
pub const GUEST_SKIP_ASK_GIVE_UP_MS: u64 = 2_000;
pub const SKIP_ASK_RESEND_MS: u64 = 500;
pub const PARTNER_SKIP_ASK_LIFETIME_MS: u64 = 2_000;
pub const PARTNER_REPORT_STALE_FOR_A_SKIP_VOTE_MS: u64 = 3_000;
pub const OWN_SKIP_ASK_LASTS_MS: u64 = 10_000;
pub const SCRIPT_PLAY_REQUEST_COUNTS_FOR_ITS_START_MS: u64 = 60_000;
pub const EVENT_STARTED_FOR_THE_PARTNER_LOAD_GIVE_UP_MS: u64 = 10_000;
pub const EVENT_STARTED_FOR_THE_PARTNER_PLAY_GIVE_UP_MS: u64 = 5_000;
pub const EVENT_STARTED_FOR_THE_PARTNER_WATCHED_MOST_MS: u64 = 600_000;
pub const EVENT_FOLLOW_WAIT_LOG_INTERVAL_MS: u64 = 1_000;
pub const MOST_SEEN_EVENTS_REMEMBERED: usize = 16;
pub const MOST_SCRIPT_PLAY_REQUESTS_REMEMBERED: usize = 16;
pub const MOST_HOST_EVENTS_REMEMBERED: usize = 16;
pub const MOST_PLAYS_HELD: usize = 8;
pub const STORY_EVENTS_BELOW_PHASE_ID: i32 = 0xF00;
pub const EVENT_ID_BYTES: usize = 12;
pub const HIGHEST_EVENT_KIND: i32 = 2;
pub const EVENT_SLOT_COUNT: usize = 8;
pub const EVENT_SLOT_STRIDE: usize = 0x60;
pub const EVENT_SLOT_TABLE_BYTES: usize = EVENT_SLOT_COUNT * EVENT_SLOT_STRIDE;
pub const EVENT_SLOT_ID_OFFSET: usize = 4;
pub const EVENT_SLOT_STATE_OFFSET: usize = 0x14;
pub const EVENT_SLOT_USED_BIT: u32 = 1;
pub const EVENT_SLOT_PLAY_WANTED_BIT: u32 = 4;
pub const EVENT_SLOT_DYING_BIT: u32 = 0x8000_0000;
pub const EVENT_SLOT_LOADING_STATE: i32 = 1;
pub const EVENT_SLOT_LOADED_STATE: i32 = 2;
pub const EVENT_SLOT_PLAYING_STATE: i32 = 3;
pub const EVENT_SLOT_LOAD_FAILED_STATE: i32 = 6;
pub const EVENT_FLAG_WORD_PLAYING_MASK: u32 = 0x6000_0000;
pub const PAUSE_STATE_TAKING_AN_EVENT_SKIP: i32 = 6;
pub const PHASE_EVENT_ACTION_DATA_ID_OFFSET: u64 = 0x50;
pub const EVENT_START_HOOK_FUNCTION_START: [u8; 11] = [
    0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x8B, 0x01, 0x48, 0x8B, 0xD9,
];
pub const EVENT_END_HOOK_FUNCTION_START: [u8; 10] =
    [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10];
pub const EVENT_SKIP_FUNCTION_START: [u8; 11] = [
    0x48, 0x8B, 0xC4, 0x57, 0x48, 0x81, 0xEC, 0x80, 0x00, 0x00, 0x00,
];
pub const EVENT_LOAD_AND_PLAY_FUNCTION_START: [u8; 13] = [
    0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x33, 0xD2, 0xE8, 0x53, 0x12, 0x00, 0x00,
];
