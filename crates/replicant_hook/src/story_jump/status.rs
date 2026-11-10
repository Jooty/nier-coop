use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};

use replicant_game::stay_together::FollowRole;
use replicant_proto::StoryJumpPoint;

use crate::net::{self, NetState};

static PENDING_JUMP: Mutex<Option<StoryJumpPoint>> = Mutex::new(None);
static GUEST_LOADOUTS: Mutex<Vec<StoryJumpPoint>> = Mutex::new(Vec::new());
static PAUSE_MENU_CLOSE_ASKED: AtomicBool = AtomicBool::new(false);
static LAST_OUTCOME: Mutex<String> = Mutex::new(String::new());

pub(super) struct PendingJump;

pub(super) struct GuestLoadouts;

impl PendingJump {
    pub(super) fn set(point: StoryJumpPoint) {
        *PENDING_JUMP.lock().unwrap_or_else(PoisonError::into_inner) = Some(point);
        PAUSE_MENU_CLOSE_ASKED.store(false, Ordering::Relaxed);
    }

    pub(super) fn take_pause_menu_close_turn() -> bool {
        !PAUSE_MENU_CLOSE_ASKED.swap(true, Ordering::Relaxed)
    }

    pub(super) fn get() -> Option<StoryJumpPoint> {
        *PENDING_JUMP.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(super) fn clear() {
        *PENDING_JUMP.lock().unwrap_or_else(PoisonError::into_inner) = None;
    }
}

impl GuestLoadouts {
    pub(super) fn push(point: StoryJumpPoint) {
        GUEST_LOADOUTS
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(point);
        PAUSE_MENU_CLOSE_ASKED.store(false, Ordering::Relaxed);
    }

    pub(super) fn take_all() -> Vec<StoryJumpPoint> {
        std::mem::take(
            &mut *GUEST_LOADOUTS
                .lock()
                .unwrap_or_else(PoisonError::into_inner),
        )
    }

    pub(super) fn waiting() -> bool {
        !GUEST_LOADOUTS
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_empty()
    }
}

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

pub(crate) fn jump_is_waiting_for_menu() -> bool {
    PendingJump::get().is_some()
}
