mod body_access;
mod driver;
mod driver_state;

pub use driver::{held_partner_body, install, tick};

const FREE_CHANGE_LINES_LOGGED_AT_MOST: u32 = 60;
const BODY_READING_LINE_INTERVAL_MS: u64 = 2_000;
const BODY_READING_LINES_LOGGED_AT_MOST: u32 = 120;
