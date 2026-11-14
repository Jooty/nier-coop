mod action_look;
mod guest_steps;
mod guest_watch;
mod host_steps;
mod observed_done;
mod step_class_lookup;
mod step_class_row;
mod step_class_table;
mod step_lines;
mod step_sort;
mod step_verdict;
mod steps_machine;
mod steps_mode;
mod steps_step;
mod story_step_answer;
mod story_step_ask;
mod story_step_done;

pub use action_look::{ActionLook, StepActionState};
pub use observed_done::ObservedDone;
pub use step_class_lookup::StepClassLookup;
pub use step_class_row::{Basis, StepClassRow};
pub use step_class_table::{step_class_table, step_class_table_rows_unread};
pub use step_lines::{
    busy_passes_line, set_done_line, story_steps_hooks_missing_line, story_steps_off_line,
    story_steps_on_line, story_steps_start_differs_line,
};
pub use step_sort::StepSort;
pub use step_verdict::StepVerdict;
pub use steps_machine::{StepsMachine, StepsReadings};
pub use steps_mode::StoryStepsMode;
pub use steps_step::{SetDone, SetDoneWhy, StepSend, StepsStep};
pub use story_step_answer::{StepAnswer, StoryStepAnswer};
pub use story_step_ask::StoryStepAsk;
pub use story_step_done::StoryStepDone;

pub const STORY_STEP_ASK_RESEND_MS: u64 = 1_000;
pub const STORY_STEP_GIVE_UP_MS: u64 = 20_000;
pub const STORY_STEP_HOST_NOT_STARTED_WAIT_MS: u64 = 3_000;
pub const WATCHED_STEP_COMPARE_AFTER_MS: u64 = 10_000;
pub const HELD_STEPS_WAIT_LINE_INTERVAL_MS: u64 = 1_000;
pub const MOST_HELD_STORY_STEPS: usize = 32;
pub const MOST_PENDING_HOST_STEPS: usize = 256;
pub const MOST_TOLD_STEPS_KEPT_AT_THE_HOST: usize = 512;
pub const MOST_WATCHED_STEPS_KEPT: usize = 256;
pub const UNMIRRORED_DONE_LINES_LOGGED_PER_MINUTE: u32 = 60;
pub const MOST_UNKNOWN_CLASSES_NAMED: usize = 128;
pub const ACTION_DONE_STATE_HASH: u32 = 0x26B8_0D1F;
pub const ACTION_ID_OFFSET: usize = 0x18;
pub const ACTION_FLAGS_OFFSET: usize = 0x1C;
pub const ACTION_STATE_HASH_OFFSET: usize = 0x20;
pub const ACTION_STATE_VALUE_OFFSET: usize = 0x30;
pub const STATE_ARGUMENT_HASH_OFFSET: usize = 0x0;
pub const STATE_ARGUMENT_VALUE_OFFSET: usize = 0x10;
pub const ACTION_HOLDS_ITS_SEQUENCE_FLAG: u32 = 0x8;
pub const ACTION_STARTED_FLAG: u32 = 0x20_0000;
pub const ACTION_DONE_FLAG: u32 = 0x80_0000;
pub const SET_ACTION_STATE_FUNCTION_START: [u8; 31] = [
    0x48, 0x89, 0x5C, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x30, 0x48, 0x8B, 0xD9, 0x48, 0x8B, 0xFA,
    0x8B, 0x0A, 0x39, 0x4B, 0x20, 0x75, 0x15, 0x8B, 0x42, 0x10, 0x39, 0x43, 0x30, 0x75, 0x0D,
];
pub const SET_ACTION_STATE_BY_HASH_FUNCTION_START: [u8; 31] = [
    0x40, 0x57, 0x48, 0x83, 0xEC, 0x40, 0x48, 0xC7, 0x44, 0x24, 0x20, 0xFE, 0xFF, 0xFF, 0xFF, 0x48,
    0x89, 0x5C, 0x24, 0x50, 0x41, 0x8B, 0xD8, 0x48, 0x8B, 0xF9, 0x48, 0x8D, 0x4C, 0x24, 0x28,
];
pub const RESET_ACTION_FUNCTION_START: [u8; 32] = [
    0x48, 0x8B, 0xC4, 0x57, 0x48, 0x83, 0xEC, 0x70, 0x48, 0xC7, 0x40, 0xA8, 0xFE, 0xFF, 0xFF, 0xFF,
    0x48, 0x89, 0x58, 0x10, 0x48, 0x89, 0x70, 0x18, 0x48, 0x8B, 0xD9, 0x48, 0x8B, 0x49, 0x08, 0xF6,
];
pub const FIND_STORY_ACTION_BY_ID_FUNCTION_START: [u8; 20] = [
    0x4C, 0x8B, 0x49, 0x48, 0x4C, 0x8B, 0xD9, 0x48, 0x8B, 0x41, 0x50, 0x4D, 0x8D, 0x14, 0xC1, 0x4D,
    0x3B, 0xCA, 0x74, 0x36,
];
