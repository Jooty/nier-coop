use std::sync::atomic::{AtomicBool, Ordering};

use crate::diagnostics::log_line;

static PAUSED_PLAYERS_CAN_BE_HIT: AtomicBool = AtomicBool::new(false);

pub fn paused_players_can_be_hit() -> bool {
    PAUSED_PLAYERS_CAN_BE_HIT.load(Ordering::Relaxed)
}

pub fn set_paused_players_can_be_hit(hittable: bool) -> bool {
    let changed = PAUSED_PLAYERS_CAN_BE_HIT.swap(hittable, Ordering::Relaxed) != hittable;
    if changed {
        log_line(&format!(
            "pause: paused players can be hit is now {}",
            if hittable { "on" } else { "off" }
        ));
    }
    changed
}
