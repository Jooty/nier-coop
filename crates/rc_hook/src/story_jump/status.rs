use std::sync::{Mutex, PoisonError};

use rc_game::stay_together::FollowRole;
use rc_proto::StoryJumpPoint;

use crate::net::{self, NetState};

static PENDING_JUMP: Mutex<Option<StoryJumpPoint>> = Mutex::new(None);
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

pub(super) struct PendingJump;

impl PendingJump {
    pub(super) fn set(point: StoryJumpPoint) {
        *PENDING_JUMP.lock().unwrap_or_else(PoisonError::into_inner) = Some(point);
    }

    pub(super) fn get() -> Option<StoryJumpPoint> {
        *PENDING_JUMP.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(super) fn clear() {
        *PENDING_JUMP.lock().unwrap_or_else(PoisonError::into_inner) = None;
    }
}

pub(crate) fn jump_is_waiting_for_menu() -> bool {
    PendingJump::get().is_some()
}
