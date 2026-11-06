use std::sync::atomic::{AtomicU16, Ordering};

use rc_game::puppet::MAX_MIRROR_DELAY_MS;

static MIRROR_DELAY_MS: AtomicU16 = AtomicU16::new(0);

pub fn set_mirror_delay_ms(delay_ms: u16) {
    MIRROR_DELAY_MS.store(delay_ms.min(MAX_MIRROR_DELAY_MS), Ordering::Relaxed);
}

pub fn mirror_delay_ms() -> u16 {
    MIRROR_DELAY_MS.load(Ordering::Relaxed)
}
