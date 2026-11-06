#[derive(Debug, Clone)]
pub struct RateWindow {
    window_ms: u64,
    limit: u32,
    window_start_ms: u64,
    admitted: u32,
    refused: u32,
}

impl RateWindow {
    pub fn new(window_ms: u64, limit: u32) -> Self {
        Self {
            window_ms,
            limit,
            window_start_ms: 0,
            admitted: 0,
            refused: 0,
        }
    }

    /// `true` while fewer than `limit` calls were admitted in the current window.
    pub fn admit(&mut self, now_ms: u64) -> bool {
        self.roll(now_ms);
        if self.admitted < self.limit {
            self.admitted += 1;
            true
        } else {
            self.refused += 1;
            false
        }
    }

    /// The number of calls refused since this was last called.
    pub fn take_refused(&mut self, now_ms: u64) -> u32 {
        self.roll(now_ms);
        core::mem::take(&mut self.refused)
    }

    fn roll(&mut self, now_ms: u64) {
        if now_ms.saturating_sub(self.window_start_ms) >= self.window_ms {
            self.window_start_ms = now_ms;
            self.admitted = 0;
        }
    }
}
