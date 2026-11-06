use crate::zone_rules::HOST_WANTS_TO_GO_COOLDOWN_MS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AttemptCooldown {
    last_sent_ms: Option<u64>,
}

impl AttemptCooldown {
    pub const fn new() -> Self {
        Self { last_sent_ms: None }
    }

    pub fn should_send(&mut self, now_ms: u64) -> bool {
        let due = self
            .last_sent_ms
            .is_none_or(|last| now_ms.saturating_sub(last) >= HOST_WANTS_TO_GO_COOLDOWN_MS);
        if due {
            self.last_sent_ms = Some(now_ms);
        }
        due
    }
}
