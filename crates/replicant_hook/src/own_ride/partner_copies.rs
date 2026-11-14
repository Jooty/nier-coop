use replicant_game::own_ride::{is_a_ride_function, is_the_riders_own};

use crate::diagnostics::log_line;

/// A copy of the partner's call that would start, run or end a ride here: never run.
pub(crate) fn drop_the_partners_ride_copy(name: &str) -> bool {
    if !is_the_riders_own(name) {
        return false;
    }
    log_line(&format!(
        "ride: the partner is riding, nothing starts here (their copy of {name} is not run, your body stays where it is)"
    ));
    true
}

/// A function of this player's own ride: it is never sent to the partner.
pub(crate) fn keep_the_ride_from_the_partner(name: &str) -> bool {
    if !is_a_ride_function(name) {
        return false;
    }
    log_line(&format!(
        "ride: {name} is your own ride, not sent to the partner"
    ));
    true
}
