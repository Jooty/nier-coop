mod body_access;
mod defeat_watch;
mod downed_watch;
mod driver;
mod driver_state;

pub use downed_watch::{ask_to_give_up, downed_view, hero_is_down};
pub use driver::{
    ask_for_partner_data, body_status_line, held_partner_body, install, partner_data_reading, tick,
};

const FREE_CHANGE_LINES_LOGGED_AT_MOST: u32 = 60;
const BODY_READING_LINE_INTERVAL_MS: u64 = 2_000;
const BODY_READING_LINES_LOGGED_AT_MOST: u32 = 120;
