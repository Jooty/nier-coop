mod body_access;
mod body_drive;
mod defeat_watch;
mod downed_watch;
mod driver;
mod driver_state;
mod flight_drive;
mod watch_drive;

pub(crate) use body_drive::BodyDrive;
pub use downed_watch::{ask_to_give_up, downed_view, hero_is_down};
pub use driver::{
    ask_for_partner_data, body_status_line, held_partner_body, install, partner_data_reading, tick,
};

const FREE_CHANGE_LINES_LOGGED_PER_MINUTE: u32 = 60;
const FREE_CHANGE_LINES_TOPIC: &str = "free";
const BODY_READING_LINE_INTERVAL_MS: u64 = 2_000;
const BODY_READING_LINES_LOGGED_PER_MINUTE: u32 = 20;
const BODY_READING_LINES_TOPIC: &str = "partner: body readings";
const WATCH_REFUSAL_LINES_LOGGED_PER_MINUTE: u32 = 30;
const WATCH_REFUSAL_LINES_TOPIC: &str = "watching: feed notes";
