use std::sync::atomic::{AtomicBool, Ordering};

static LOCAL_UNATTENDED: AtomicBool = AtomicBool::new(false);

/// Whether every state message of this game says it is unattended from now on.
pub fn set_local_unattended(unattended: bool) {
    LOCAL_UNATTENDED.store(unattended, Ordering::Relaxed);
}

pub fn local_unattended() -> bool {
    LOCAL_UNATTENDED.load(Ordering::Relaxed)
}
