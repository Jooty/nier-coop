use std::sync::atomic::{AtomicBool, Ordering};

static LOCAL_WATCHING: AtomicBool = AtomicBool::new(false);

/// Whether every state message of this game says it watches the host from now on.
pub fn set_local_watching(watching: bool) {
    LOCAL_WATCHING.store(watching, Ordering::Relaxed);
}

pub fn local_watching() -> bool {
    LOCAL_WATCHING.load(Ordering::Relaxed)
}
