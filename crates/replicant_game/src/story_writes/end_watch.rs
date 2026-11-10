use crate::story_writes::END_WATCH_SETTLE_MS;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EndWatch {
    ended_ms: Option<u64>,
}

impl EndWatch {
    pub const fn new() -> Self {
        Self { ended_ms: None }
    }

    pub fn begin(&mut self, now_ms: u64) {
        self.ended_ms = Some(now_ms);
    }

    pub fn take_if_settled(&mut self, now_ms: u64) -> bool {
        let settled = self
            .ended_ms
            .is_some_and(|ended| now_ms.saturating_sub(ended) >= END_WATCH_SETTLE_MS);
        if settled {
            self.ended_ms = None;
        }
        settled
    }

    pub fn clear(&mut self) {
        self.ended_ms = None;
    }
}
