mod hero_controls;
mod start_hero;
mod state_difference;
mod watch_feed;
mod watch_lines;
mod watch_machine;
mod watch_role;
mod watched_flight;
mod watched_flight_lines;
mod watched_stretch;

pub use hero_controls::{
    HERO_INPUT_SOURCE_OFF, HERO_INPUT_SOURCE_ON, HeroInputSources, hero_fields_given_back,
    hero_fields_while_watched,
};
pub use start_hero::hero_written_for_a_start;
pub use state_difference::{StateDifference, state_agrees_again_line, state_differs_line};
pub use watch_feed::{WatchFeedReadings, WatchFeedRefusal, watch_feed_refusal};
pub use watch_lines::{
    WatchingReading, hand_back_line, hero_teleported_line, hero_was_held_line,
    host_defeat_seen_while_watching_line, host_defeated_while_watched_line, host_watched_line,
    own_hits_kept_back_line, own_pickups_kept_back_line, start_hero_line, watch_refusal_line,
    watching_ended_by_the_bound_line, watching_ended_by_the_load_line,
    watching_ended_without_the_host_line, watching_goes_on_outside_line, watching_line,
    watching_setting_line, watching_started_line,
};
pub use watch_machine::{FullLoadVerdict, GuestWatch, WatchReadings, WatchStep};
pub use watch_role::{WatchRole, guest_watch_applies, host_is_watched};
pub use watched_flight::{
    HostFlight, WatchedFlightReadings, WatchedFlightStep, WatchedHeroFlight,
    WatchedHeroFlightReading,
};
pub use watched_flight_lines::{
    flight_call_label, flight_report_label, hero_flight_after_call_line,
    hero_flight_mode_left_alone_line, hero_flight_refused_line, hero_flight_try_line,
    hero_stays_in_its_shooting_stage_line, hero_unit_follows_line,
    host_flight_report_is_empty_line, watched_hero_flight_setting_line, watching_flight_part,
};
pub use watched_stretch::{WATCHED_PHASE_ID, WATCHED_SUB_PHASE_MARK, WatchedStretch};

pub const WATCHED_HERO_STATE_MAY_DIFFER_FOR_MS: u64 = 2_000;
pub const HAND_BACK_LOAD_MAY_TAKE_MS: u64 = 120_000;
pub const WATCHING_LINE_INTERVAL_MS: u64 = 1_000;
pub const WATCHED_HERO_FLIGHT_FOLLOWS_AFTER_MS: u64 = 1_500;
pub const WATCHED_HERO_FLIGHT_TRIES: u32 = 3;
pub const WATCHED_HERO_FLIGHT_RETRY_MS: u64 = 1_000;
