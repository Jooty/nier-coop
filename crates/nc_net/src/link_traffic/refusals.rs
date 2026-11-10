use std::collections::HashMap;
use std::time::Instant;

use crate::link_traffic::REFUSAL_LOG_INTERVAL;

#[derive(Clone, Debug, Default)]
pub struct RefusalLog {
    last_logged: HashMap<&'static str, Instant>,
}

impl RefusalLog {
    pub fn should_log(&mut self, kind: &'static str, now: Instant) -> bool {
        if let Some(last) = self.last_logged.get(kind)
            && now.saturating_duration_since(*last) < REFUSAL_LOG_INTERVAL
        {
            return false;
        }
        self.last_logged.insert(kind, now);
        true
    }
}
