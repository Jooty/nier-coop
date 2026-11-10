use crate::effects::RateWindow;

pub const PROBE_LOG_LINES_PER_WINDOW: u32 = 40;
pub const PROBE_LOG_WINDOW_MS: u64 = 10_000;

#[derive(Debug, Clone)]
pub struct ProbeLogLimiter {
    window: RateWindow,
}

impl Default for ProbeLogLimiter {
    fn default() -> Self {
        Self {
            window: RateWindow::new(PROBE_LOG_WINDOW_MS, PROBE_LOG_LINES_PER_WINDOW),
        }
    }
}

impl ProbeLogLimiter {
    /// `None` when the line is dropped; otherwise the number of lines dropped since the last admitted one.
    pub fn admit(&mut self, now_ms: u64) -> Option<u32> {
        if self.window.admit(now_ms) {
            Some(self.window.take_refused(now_ms))
        } else {
            None
        }
    }
}
