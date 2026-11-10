use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq)]
pub struct RemotePlayer<P> {
    pub name: String,
    pub slot: u8,
    pub state: P,
    pub received_at: Instant,
}

impl<P> RemotePlayer<P> {
    pub fn age(&self) -> Duration {
        self.received_at.elapsed()
    }
}
