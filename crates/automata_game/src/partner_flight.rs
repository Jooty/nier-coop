mod body_flight;
mod bounded_tries;
mod flight_fields;
mod flight_kind;
mod flight_lines;
mod flight_machine;
mod flight_mode;
mod flight_report;
mod flight_states;
mod flight_step;
mod riding_mismatch;
mod stick_choice;
mod unit_placer;

pub use body_flight::BodyFlight;
pub use bounded_tries::{BoundedTries, TryVerdict};
pub use flight_fields::{FlightFields, UnitPose, flight_report_of};
pub use flight_kind::FlightKind;
pub use flight_lines::{
    board_refused_line, board_try_line, body_got_off_line, body_rides_line,
    free_flight_mode_left_alone_line, get_off_before_release_line, get_off_refused_line,
    get_off_try_line, mode_refused_line, mode_try_line, partner_flight_unit_setting_line,
    rider_moved_without_teleport_line, riding_agrees_again_line, riding_mismatch_line,
    unit_placed_line,
};
pub use flight_machine::{FlightReadings, PartnerFlight, gets_off_before_release};
pub use flight_mode::FlightMode;
pub use flight_report::FlightReport;
pub use flight_states::{is_a_flight_state, is_a_shooting_stage_state};
pub use flight_step::{FlightCall, FlightStep};
pub use riding_mismatch::RidingMismatch;
pub use stick_choice::{StickChoice, flight_stick_in_use};
pub use unit_placer::UnitPlacer;

pub const PARTNER_BOARD_RETRY_MS: u64 = 1_000;
pub const PARTNER_BOARD_TRIES: u32 = 5;
pub const PARTNER_MODE_RETRY_MS: u64 = 1_000;
pub const PARTNER_MODE_TRIES: u32 = 5;
pub const PARTNER_GET_OFF_RETRY_MS: u64 = 1_000;
pub const PARTNER_GET_OFF_TRIES: u32 = 5;
pub const PARTNER_OUT_OF_A_UNIT_BEFORE_THE_BODY_GETS_OFF_MS: u64 = 1_000;
pub const RIDING_MISMATCH_LOGGED_AFTER_MS: u64 = 1_000;

pub const FIRST_FLIGHT_STATE: i32 = 0x70;
pub const LAST_FLIGHT_STATE: i32 = 0x9F;
pub const FIRST_SHOOTING_STAGE_STATE: i32 = 0x84;
pub const LAST_SHOOTING_STAGE_STATE: i32 = 0x92;

pub const FLIGHT_UNIT_POSITION_SENT_AT_MOST: f32 = 1_000_000.0;
pub const FLIGHT_STICK_AXIS_SENT_AT_MOST: f32 = 1_000.0;

pub const BODY_RIDING_FLIGHT_UNIT_OFFSET: u32 = 0x1_5B60;
pub const BODY_FLIGHT_UNIT_HANDLE_OFFSET: u32 = 0x1_5B64;
pub const BODY_SHOOTING_STAGE_OFFSET: u32 = 0x1_6A50;
pub const BODY_FLIGHT_MODE_FIGHTER_OFFSET: u32 = 0x1_6A70;
pub const FLIGHT_UNIT_POSITION_OFFSET: u32 = 0x50;
pub const FLIGHT_UNIT_YAW_OFFSET: u32 = 0x94;
pub const CONTROLLER_PAD_STICK_OFFSET: u32 = 0x60;
pub const CONTROLLER_MOUSE_STICK_OFFSET: u32 = 0x274;
pub const CONTROLLER_MOUSE_STICK_LENGTH_OFFSET: u32 = 0x2E0;
pub const CONTROLLER_KEYBOARD_STICK_OFFSET: u32 = 0x698;
pub const CONTROLLER_KEYBOARD_STICK_LENGTH_OFFSET: u32 = 0x700;
pub const PAD_ENABLED: i32 = 1;
