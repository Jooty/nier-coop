use std::sync::{Mutex, PoisonError};

use rc_game::stay_together::FollowRole;

use crate::net::{self, NetState};

static LAST_OUTCOME: Mutex<String> = Mutex::new(String::new());

pub(crate) fn host_can_offer_jump() -> bool {
    let Some(net) = net::handle() else {
        return false;
    };
    let view = net.view();
    view.state == NetState::Connected && FollowRole::from_slot(view.local_slot) == FollowRole::Host
}

pub(crate) fn last_refusal() -> String {
    LAST_OUTCOME
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

pub(super) fn remember_outcome(text: String) {
    *LAST_OUTCOME.lock().unwrap_or_else(PoisonError::into_inner) = text;
}
