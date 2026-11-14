use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::net::STATE_TICK_CLOCK_STEP_MS;

/// Where a worker's state count starts: later for every later start, so the other side never
/// takes the states of a game that was started again for old ones.
pub fn first_state_tick(since_unix_epoch: Duration) -> u32 {
    // The count wraps like the receiver's comparison does.
    (since_unix_epoch.as_millis() / STATE_TICK_CLOCK_STEP_MS) as u32
}

pub(super) fn first_state_tick_now() -> u32 {
    first_state_tick(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default(),
    )
}
