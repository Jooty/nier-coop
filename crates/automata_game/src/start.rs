mod candidates;
mod chapter_record;
mod chapter_table;
mod flag_names;
mod start_config;
mod start_machine;
mod start_plan;
mod start_readings;

pub use candidates::{Candidate, candidates, table_log_lines};
pub use chapter_record::ChapterRecord;
pub use chapter_table::{ChapterTableHeader, parse_records};
pub use flag_names::{buddy_flag, hero_flag, hero_mask_has, route_flag};
pub use start_config::{StartConfig, StartMode};
pub use start_machine::{StartAction, StartMachine, StartState, StartStep};
pub use start_plan::{PhaseStartCall, StartCall, StartPlan, build_plan};
pub use start_readings::{
    ArrivalReadings, StartReadings, TitleReadings, has_arrived, title_is_idle,
};

pub const CHAPTER_RECORD_SIZE: usize = 0x50;
pub const CHAPTER_HEADER_READ_LEN: usize = 0x12;
pub const MAX_CHAPTER_RECORDS: usize = 1024;
pub const CONDITIONAL_GROUP: u8 = 0x11;
pub const OPENING_MISSION_PHASE_ID: i16 = 0x100;
pub const OPENING_MISSION_NAME_PART: &str = "Prologue";
pub const BASE_NAME_PART: &str = "Bunker";
pub const TITLE_SCENE_STATE: i32 = 3;
pub const TITLE_PHASE_ID: i32 = 0xF00;
pub const NO_PENDING_PHASE_ID: i32 = -1;
pub const PHASE_START_RETRY_LIMIT_MS: u64 = 20_000;
pub const POLL_LOG_INTERVAL_MS: u64 = 10_000;
pub const SAVE_BUSY_SETTLE_MS: u64 = 3_000;
pub const UI_WORD_MASK_WITHOUT_TOP_BIT: u32 = 0x7FFF_FFFF;
