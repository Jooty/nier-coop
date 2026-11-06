use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug)]
pub struct TakeOnce(AtomicBool);

impl TakeOnce {
    pub const fn new() -> Self {
        Self(AtomicBool::new(false))
    }

    pub fn request(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn clear(&self) {
        self.0.store(false, Ordering::Relaxed);
    }

    pub fn take(&self) -> bool {
        self.0.swap(false, Ordering::Relaxed)
    }
}

impl Default for TakeOnce {
    fn default() -> Self {
        Self::new()
    }
}
