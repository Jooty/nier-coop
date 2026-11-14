mod driver;
mod driver_state;
mod lead_state;
mod lead_stream;
mod spawn_share;
mod unforced_defeats;

pub use driver::{install, tick};

const CENSUS_SUMMARY_LINE_INTERVAL_MS: u64 = 5_000;
const MISMATCH_LINES_LOGGED_PER_MINUTE: u32 = 60;
const SHARE_LINES_LOGGED_PER_MINUTE: u32 = 60;
const SPAWN_LINES_LOGGED_PER_MINUTE: u32 = 60;
const UNPLACED_ENEMY_LINES_LOGGED_PER_MINUTE: u32 = 60;
const MOST_GENERATOR_STATES_NAMED_IN_ONE_LINE: usize = 4;
