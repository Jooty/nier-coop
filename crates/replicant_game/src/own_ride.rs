//! A ride belongs to the player whose own game started it: the partner's game never starts,
//! joins, pauses for or skips it, and sees the rider's body move by the body sync.
mod dropped_step;
mod names;
mod ride_watch;

pub use dropped_step::{DroppedRideStep, DroppedStepRelease, DroppedStepView};
pub use names::{is_a_ride_function, is_the_riders_own, leads_into_the_ride};
pub use ride_watch::{RideChange, RideWatch};

pub const RIDE_STARTED_BY: &str = "Put_Boat_02";
pub const RIDE_ENDED_BY: [&str; 3] = ["Put_Boat_03", "Put_Boat_04", "Put_Boat_05"];
pub const RIDE_FUNCTIONS: [&str; 5] = [
    "Put_Boat_02",
    "Put_Boat_03",
    "Put_Boat_04",
    "Put_Boat_05",
    "Put_Boat_06",
];
pub const STEPS_THAT_LEAD_INTO_THE_RIDE: [&str; 12] = [
    "EID_1410_a0100g_05",
    "EID_1410_a0100g_05_01",
    "EID_1410_a0100g_06",
    "EID_1410_a0100g_10_Motion02",
    "EID_1410_a0100g_11",
    "EID_1410_a0100g_16_Motion02",
    "EID_1410_a0100g_17",
    "EID_1410_a0100g_21_04",
    "EID_1410_a0100g_27_Motion02",
    "EID_1410_a0100g_28",
    "EID_1410_a0100g_32_Motion02",
    "EID_1410_a0100g_33",
];
pub const RIDE_END_MOVES_THE_PLAYER_FOR_MS: u64 = 10_000;
pub const DROPPED_RIDE_STEP_SETTLES_AFTER_MS: u64 = 4_000;
pub const DROPPED_RIDE_STEP_WATCHED_FOR_MS: u64 = 120_000;
pub const END_UNIQUE_ACTOR_SCRIPT_MODE_NATIVE_RVA: u32 = 0x6E_8BC0;
pub const SET_PLAYER_DEFAULT_STATUS_NATIVE_RVA: u32 = 0x6F_EE90;
