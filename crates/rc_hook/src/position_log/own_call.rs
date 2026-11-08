use std::sync::atomic::{AtomicBool, Ordering};

static OWN_CALL_RUNNING: AtomicBool = AtomicBool::new(false);

pub(crate) struct OwnCall;

impl OwnCall {
    pub(crate) fn begin() -> Self {
        OWN_CALL_RUNNING.store(true, Ordering::Relaxed);
        Self
    }
}

impl Drop for OwnCall {
    fn drop(&mut self) {
        OWN_CALL_RUNNING.store(false, Ordering::Relaxed);
    }
}

pub(super) fn own_call_is_running() -> bool {
    OWN_CALL_RUNNING.load(Ordering::Relaxed)
}
