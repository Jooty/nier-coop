mod driver;
mod driver_state;

pub use driver::{install, tick};

const CENSUS_SUMMARY_LINE_INTERVAL_MS: u64 = 5_000;
const MISMATCH_LINES_LOGGED_AT_MOST: u32 = 300;
