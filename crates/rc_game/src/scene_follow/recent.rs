use std::collections::VecDeque;

use crate::scene_follow::SCENE_FOLLOW_RECENT_CAPACITY;
use crate::story_calls::is_event_callback_name;

#[derive(Debug, Clone, Default)]
pub struct RecentCallbacks {
    fired: VecDeque<(String, u64)>,
}

impl RecentCallbacks {
    pub const fn new() -> Self {
        Self {
            fired: VecDeque::new(),
        }
    }

    pub fn note(&mut self, name: &str, now_ms: u64) {
        if !is_event_callback_name(name) {
            return;
        }
        if self.fired.len() >= SCENE_FOLLOW_RECENT_CAPACITY {
            self.fired.pop_front();
        }
        self.fired.push_back((name.to_owned(), now_ms));
    }

    pub fn ms_since_fired(&self, name: &str, now_ms: u64) -> Option<u64> {
        self.fired
            .iter()
            .rev()
            .find(|(fired, _)| fired == name)
            .map(|(_, fired_ms)| now_ms.saturating_sub(*fired_ms))
    }

    pub fn scene_name(&self, slot_holder: Option<&str>) -> Option<String> {
        slot_holder
            .filter(|holder| is_event_callback_name(holder))
            .map(str::to_owned)
            .or_else(|| self.fired.back().map(|(name, _)| name.clone()))
    }
}
