use crate::boss_messages::{MESSAGE_DEDUPE_MAX_ENTRIES, MESSAGE_DEDUPE_WINDOW_MS};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MessageDedupe {
    recent: Vec<(u32, u64)>,
}

impl MessageDedupe {
    pub const fn new() -> Self {
        Self { recent: Vec::new() }
    }

    pub fn allow(&mut self, message: u32, now_ms: u64) -> bool {
        self.recent
            .retain(|(_, at)| now_ms.saturating_sub(*at) < MESSAGE_DEDUPE_WINDOW_MS);
        if self.recent.iter().any(|(seen, _)| *seen == message) {
            return false;
        }
        if self.recent.len() >= MESSAGE_DEDUPE_MAX_ENTRIES {
            self.recent.remove(0);
        }
        self.recent.push((message, now_ms));
        true
    }

    pub fn clear(&mut self) {
        self.recent.clear();
    }
}
