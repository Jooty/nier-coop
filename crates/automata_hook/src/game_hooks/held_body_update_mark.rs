use std::sync::atomic::{AtomicU32, Ordering};

static HELD_BODY_UPDATES_RUNNING: AtomicU32 = AtomicU32::new(0);

/// Runs `update` marked as a held body's update; game thread only.
pub fn while_a_held_body_update_runs<T>(update: impl FnOnce() -> T) -> T {
    HELD_BODY_UPDATES_RUNNING.fetch_add(1, Ordering::Relaxed);
    let result = update();
    HELD_BODY_UPDATES_RUNNING.fetch_sub(1, Ordering::Relaxed);
    result
}

pub fn a_held_body_update_runs() -> bool {
    HELD_BODY_UPDATES_RUNNING.load(Ordering::Relaxed) > 0
}
