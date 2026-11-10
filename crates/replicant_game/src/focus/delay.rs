pub const FOCUS_LOSS_CLEAR_DELAY_MS: u64 = 1000;

#[derive(Debug, Default)]
pub struct FocusLossTracker {
    lost_at_ms: Option<u64>,
}

impl FocusLossTracker {
    pub fn lose(&mut self, now_ms: u64) {
        self.lost_at_ms.get_or_insert(now_ms);
    }

    pub fn regain(&mut self) {
        self.lost_at_ms = None;
    }

    pub fn is_pending(&self) -> bool {
        self.lost_at_ms.is_some()
    }

    pub fn take_due(&mut self, now_ms: u64) -> bool {
        let due = self
            .lost_at_ms
            .is_some_and(|lost| now_ms.saturating_sub(lost) > FOCUS_LOSS_CLEAR_DELAY_MS);
        if due {
            self.lost_at_ms = None;
        }
        due
    }
}
