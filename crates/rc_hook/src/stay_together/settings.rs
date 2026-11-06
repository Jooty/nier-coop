use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Mutex, PoisonError};

use rc_game::stay_together::{DEFAULT_LEASH_UNITS, Leash};

static FOLLOW_ENABLED: AtomicBool = AtomicBool::new(true);
static GO_TO_PARTNER_REQUESTED: AtomicBool = AtomicBool::new(false);
static LEASH_UNITS_BITS: AtomicU32 = AtomicU32::new(DEFAULT_LEASH_UNITS.to_bits());
static STATUS: Mutex<StayStatus> = Mutex::new(StayStatus::new());

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StayStatus {
    pub follow: String,
    pub go_to_partner: String,
    pub blocked_exits: u32,
}

impl StayStatus {
    pub const fn new() -> Self {
        Self {
            follow: String::new(),
            go_to_partner: String::new(),
            blocked_exits: 0,
        }
    }
}

pub fn follow_enabled() -> bool {
    FOLLOW_ENABLED.load(Ordering::Relaxed)
}

pub fn set_follow_enabled(enabled: bool) {
    FOLLOW_ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn leash() -> Leash {
    Leash::new(f32::from_bits(LEASH_UNITS_BITS.load(Ordering::Relaxed)))
}

pub fn set_leash(units: f32) {
    LEASH_UNITS_BITS.store(Leash::new(units).units().to_bits(), Ordering::Relaxed);
}

pub fn request_go_to_partner() {
    GO_TO_PARTNER_REQUESTED.store(true, Ordering::Relaxed);
}

pub(super) fn take_go_to_partner_request() -> bool {
    GO_TO_PARTNER_REQUESTED.swap(false, Ordering::Relaxed)
}

pub fn stay_status() -> StayStatus {
    STATUS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

pub(super) fn update_status(change: impl FnOnce(&mut StayStatus)) {
    change(&mut STATUS.lock().unwrap_or_else(PoisonError::into_inner));
}

pub fn note_blocked_exit() {
    update_status(|status| status.blocked_exits = status.blocked_exits.saturating_add(1));
}
