use crate::puppet::BODY_RESEND_INTERVAL_MS;

#[derive(Debug, Clone, Copy, Default)]
pub struct BookAnnouncer {
    last_sent: Option<(bool, u64)>,
}

impl BookAnnouncer {
    pub const fn new() -> Self {
        Self { last_sent: None }
    }

    pub fn is_due(&mut self, has_book: bool, now_ms: u64) -> bool {
        let due = match self.last_sent {
            None => true,
            Some((sent, _)) if sent != has_book => true,
            Some((_, at_ms)) => now_ms.saturating_sub(at_ms) >= BODY_RESEND_INTERVAL_MS,
        };
        if due {
            self.last_sent = Some((has_book, now_ms));
        }
        due
    }
}
