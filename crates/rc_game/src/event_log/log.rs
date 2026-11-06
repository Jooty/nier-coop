use std::collections::VecDeque;

use crate::event_log::{
    EVENT_LOG_DUPLICATE_WINDOW_MS, EVENT_LOG_FADE_IN_MS, EVENT_LOG_FADE_OUT_MS,
    EVENT_LOG_MAX_ENTRIES, EVENT_LOG_MAX_TEXT_CHARS, EVENT_LOG_VISIBLE_MS,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    text: String,
    posted_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisibleEvent {
    pub text: String,
    pub alpha: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EventLog {
    entries: VecDeque<Entry>,
}

impl EventLog {
    pub const fn new() -> Self {
        Self {
            entries: VecDeque::new(),
        }
    }

    pub fn post(&mut self, text: &str, now_ms: u64) -> bool {
        let text: String = text
            .chars()
            .filter(|character| !character.is_control())
            .take(EVENT_LOG_MAX_TEXT_CHARS)
            .collect();
        if text.is_empty() {
            return false;
        }
        let repeated = self.entries.iter().any(|entry| {
            entry.text == text
                && now_ms.saturating_sub(entry.posted_ms) < EVENT_LOG_DUPLICATE_WINDOW_MS
        });
        if repeated {
            return false;
        }
        if self.entries.len() >= EVENT_LOG_MAX_ENTRIES {
            self.entries.pop_front();
        }
        self.entries.push_back(Entry {
            text,
            posted_ms: now_ms,
        });
        true
    }

    pub fn visible(&mut self, now_ms: u64) -> Vec<VisibleEvent> {
        self.entries
            .retain(|entry| now_ms.saturating_sub(entry.posted_ms) < EVENT_LOG_VISIBLE_MS);
        self.entries
            .iter()
            .map(|entry| VisibleEvent {
                text: entry.text.clone(),
                alpha: alpha_at(now_ms.saturating_sub(entry.posted_ms)),
            })
            .collect()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

fn alpha_at(age_ms: u64) -> f32 {
    let fade_out_start = EVENT_LOG_VISIBLE_MS - EVENT_LOG_FADE_OUT_MS;
    let fade_in = (age_ms as f32 / EVENT_LOG_FADE_IN_MS as f32).min(1.0);
    let fade_out = if age_ms <= fade_out_start {
        1.0
    } else {
        (EVENT_LOG_VISIBLE_MS.saturating_sub(age_ms)) as f32 / EVENT_LOG_FADE_OUT_MS as f32
    };
    (fade_in * fade_out).clamp(0.0, 1.0)
}
