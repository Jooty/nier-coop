use crate::own_ride::{RIDE_FUNCTIONS, STEPS_THAT_LEAD_INTO_THE_RIDE};

/// A function of the ride itself: its start, its arrival, skip and trap callbacks, its camera.
pub fn is_a_ride_function(name: &str) -> bool {
    RIDE_FUNCTIONS.contains(&name)
}

/// A step that walks the player onto the ride and boards it (inline, as its last call). Its
/// story flags still reach the partner through the local scene watch.
pub fn leads_into_the_ride(name: &str) -> bool {
    STEPS_THAT_LEAD_INTO_THE_RIDE.contains(&name)
}

/// Runs only in the game of the player who takes the ride: never relayed, and a copy of it from
/// the partner is never run.
pub fn is_the_riders_own(name: &str) -> bool {
    is_a_ride_function(name) || leads_into_the_ride(name)
}
