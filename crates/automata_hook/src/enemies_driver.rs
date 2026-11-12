mod driver;
mod driver_state;
mod lead_state;
mod lead_stream;
mod spawn_share;

pub use driver::{install, tick};

const CENSUS_SUMMARY_LINE_INTERVAL_MS: u64 = 5_000;
const MISMATCH_LINES_LOGGED_AT_MOST: u32 = 300;
const SHARE_LINES_LOGGED_AT_MOST: u32 = 300;
const SPAWN_LINES_LOGGED_AT_MOST: u32 = 600;
const MOST_GENERATOR_STATES_NAMED_IN_ONE_LINE: usize = 4;
