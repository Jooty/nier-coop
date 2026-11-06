use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};

use rc_game::downed::DownedState;

pub(super) static GO_DOWN_REQUESTED: AtomicBool = AtomicBool::new(false);
pub(super) static REVIVE_REQUESTED: AtomicBool = AtomicBool::new(false);
static STATUS: Mutex<DownedStatus> = Mutex::new(DownedStatus {
    session_active: false,
    local: DownedState::Up,
    partner: DownedState::Up,
    revive_progress: 0.0,
    bleed_out_progress: 0.0,
    game_over: false,
    rewinds: 0,
});

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DownedStatus {
    pub session_active: bool,
    pub local: DownedState,
    pub partner: DownedState,
    pub revive_progress: f32,
    pub bleed_out_progress: f32,
    pub game_over: bool,
    pub rewinds: u32,
}

pub fn downed_status() -> DownedStatus {
    *STATUS.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn request_go_down() {
    GO_DOWN_REQUESTED.store(true, Ordering::Relaxed);
}

pub fn request_revive_partner() {
    REVIVE_REQUESTED.store(true, Ordering::Relaxed);
}

pub(super) fn publish(status: DownedStatus) {
    *STATUS.lock().unwrap_or_else(PoisonError::into_inner) = status;
}
