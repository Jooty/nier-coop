#[derive(Debug, Clone, Copy)]
pub struct RetryGate {
    interval_ms: u64,
    next_at_ms: Option<u64>,
}

impl RetryGate {
    pub const fn new(interval_ms: u64) -> Self {
        Self {
            interval_ms,
            next_at_ms: None,
        }
    }

    pub fn is_due(&mut self, now_ms: u64) -> bool {
        if self.next_at_ms.is_some_and(|at| now_ms < at) {
            return false;
        }
        self.next_at_ms = Some(now_ms + self.interval_ms);
        true
    }
}
