use crate::partner_flight::{FlightKind, FlightMode, RIDING_MISMATCH_LOGGED_AFTER_MS};

pub fn partner_flight_unit_setting_line(on: bool) -> String {
    if on {
        "partner flight: on (setting partner_flight_unit); in the campaign the partner body rides a flight unit while the other player's hero is in one".to_owned()
    } else {
        "partner flight: off (setting partner_flight_unit off); the partner body is driven on foot as before".to_owned()
    }
}

pub fn board_try_line(kind: FlightKind, try_number: u32, most: u32) -> String {
    format!(
        "partner flight: the other player's hero is in a flight unit ({}), the body is not; called startPhaseEventFlightUnit on the body (try {try_number} of {most})",
        kind.label()
    )
}

pub fn board_refused_line(most: u32) -> String {
    format!(
        "partner flight: the body did not get into a flight unit after {most} tries; no more tries until the other player's hero gets off and on again"
    )
}

pub fn body_rides_line(state: i32) -> String {
    format!("partner flight: the body rides its flight unit (state {state:#X})")
}

pub fn body_got_off_line(state: i32) -> String {
    format!("partner flight: the body is off its flight unit (state {state:#X})")
}

pub fn mode_try_line(
    mode: FlightMode,
    body_mode: FlightMode,
    state: i32,
    try_number: u32,
    most: u32,
) -> String {
    format!(
        "partner flight: the other player's hero is in a shooting stage in mode {}, the body in mode {} and state {state:#X}; called the {} mode setter on the body (try {try_number} of {most})",
        mode.label(),
        body_mode.label(),
        mode.label()
    )
}

pub fn mode_refused_line(mode: FlightMode, most: u32) -> String {
    format!(
        "partner flight: the body did not take mode {} in a shooting stage state after {most} tries; no more tries until the other player's mode or flight kind changes",
        mode.label()
    )
}

pub fn free_flight_mode_left_alone_line(mode: FlightMode, body_mode: FlightMode) -> String {
    format!(
        "partner flight: in free flight the other player's hero is in mode {} and the body in mode {}; no mode setter is called outside a shooting stage (each one starts state 0x84)",
        mode.label(),
        body_mode.label()
    )
}

pub fn get_off_try_line(try_number: u32, most: u32) -> String {
    format!(
        "partner flight: the other player's hero is out of its flight unit, the body still rides; called endPhaseEventFlightUnit on the body (try {try_number} of {most})"
    )
}

pub fn get_off_refused_line(most: u32) -> String {
    format!(
        "partner flight: the body still rides after {most} tries to get it off; no more tries until the other player's hero gets on and off again"
    )
}

pub fn get_off_before_release_line() -> String {
    "partner flight: the body rides a flight unit and is about to be released; called endPhaseEventFlightUnit on it first".to_owned()
}

pub fn riding_mismatch_line(hero_in_a_unit: bool, body_rides: bool) -> String {
    format!(
        "partner flight: for {RIDING_MISMATCH_LOGGED_AFTER_MS} ms the hero here is {} and the partner body {}; both set the game's riding bit, what a mismatch does is unproven",
        if hero_in_a_unit {
            "in a flight unit"
        } else {
            "not in a flight unit"
        },
        if body_rides {
            "rides one"
        } else {
            "does not ride one"
        }
    )
}

pub fn riding_agrees_again_line(lasted_ms: u64) -> String {
    format!(
        "partner flight: the hero here and the partner body agree again on riding a flight unit (they differed for {lasted_ms} ms)"
    )
}

pub fn unit_placed_line(teleport_sized: bool) -> String {
    format!(
        "partner flight: first write of the body's unit position toward the other player's unit ({}; that a plain write moves the unit is a guess until run)",
        if teleport_sized {
            "the whole way, the error was above the teleport distance"
        } else {
            "a nudge"
        }
    )
}

pub fn rider_moved_without_teleport_line() -> String {
    "partner flight: the riding body was more than a teleport's distance from its owner; moved by a plain position write, not the game's teleport, so it stays in its flight state (logged once)".to_owned()
}
