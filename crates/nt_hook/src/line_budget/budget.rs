#[derive(Debug, Clone, Default)]
pub struct LineBudget {
    window_start_ms: Option<u64>,
    admitted: u32,
    refused: u32,
}

impl LineBudget {
    pub const fn new() -> Self {
        Self {
            window_start_ms: None,
            admitted: 0,
            refused: 0,
        }
    }

    pub const fn counting_from_zero() -> Self {
        Self {
            window_start_ms: Some(0),
            admitted: 0,
            refused: 0,
        }
    }

    pub fn admit(&mut self, now_ms: u64, window_ms: u64, lines: u32) -> bool {
        self.roll(now_ms, window_ms);
        if self.admitted < lines {
            self.admitted += 1;
            true
        } else {
            self.refused = self.refused.saturating_add(1);
            false
        }
    }

    pub fn due(&mut self, now_ms: u64, gap_ms: u64) -> bool {
        self.admit(now_ms, gap_ms, 1)
    }

    pub fn take_refused(&mut self, now_ms: u64, window_ms: u64) -> u32 {
        self.roll(now_ms, window_ms);
        core::mem::take(&mut self.refused)
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    fn roll(&mut self, now_ms: u64, window_ms: u64) {
        if self
            .window_start_ms
            .is_none_or(|start| now_ms.saturating_sub(start) >= window_ms)
        {
            self.window_start_ms = Some(now_ms);
            self.admitted = 0;
        }
    }
}
