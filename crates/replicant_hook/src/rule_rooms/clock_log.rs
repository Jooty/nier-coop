use std::sync::PoisonError;

use replicant_game::stay_together::FollowRole;

use crate::diagnostics::log_line;
use crate::net::{self, net_clock_ms};
use crate::rule_rooms::state::CLOCK;

pub(super) fn tick() {
    let Some(net) = net::handle() else {
        return;
    };
    let player = match FollowRole::from_slot(net.view().local_slot) {
        FollowRole::Host => replicant_game::rule_rooms::RulePlayer::Host,
        FollowRole::Guest => replicant_game::rule_rooms::RulePlayer::Guest,
        FollowRole::Unknown => return,
    };
    let line = CLOCK
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .line_if_due(net_clock_ms(), player);
    if let Some(line) = line {
        log_line(&line);
    }
}
