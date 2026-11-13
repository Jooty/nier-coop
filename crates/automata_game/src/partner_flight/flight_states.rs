use crate::partner_flight::{
    FIRST_FLIGHT_STATE, FIRST_SHOOTING_STAGE_STATE, LAST_FLIGHT_STATE, LAST_SHOOTING_STAGE_STATE,
};

/// A state of the rider's own flight state machine, which its game runs from its fed input.
pub fn is_a_flight_state(state: i32) -> bool {
    (FIRST_FLIGHT_STATE..=LAST_FLIGHT_STATE).contains(&state)
}

pub fn is_a_shooting_stage_state(state: i32) -> bool {
    (FIRST_SHOOTING_STAGE_STATE..=LAST_SHOOTING_STAGE_STATE).contains(&state)
}
