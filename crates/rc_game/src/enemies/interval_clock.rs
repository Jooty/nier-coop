#[derive(Debug, Default)]
pub struct IntervalClock {
    last_ms: Option<u64>,
}

impl IntervalClock {
    pub const fn new() -> Self {
        Self { last_ms: None }
    }

    pub fn due(&mut self, now_ms: u64, interval_ms: u64) -> bool {
        if self
            .last_ms
            .is_some_and(|last| now_ms.saturating_sub(last) < interval_ms)
        {
            return false;
        }
        self.last_ms = Some(now_ms);
        true
    }

    pub fn reset(&mut self) {
        self.last_ms = None;
    }
}
