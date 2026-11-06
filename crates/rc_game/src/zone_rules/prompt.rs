use crate::zone_rules::{VETO_LOG_INTERVAL_MS, VETO_PROMPT_VISIBLE_MS};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VetoNotice {
    text: String,
    visible_until_ms: u64,
    logged: Option<(String, u64)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VetoRecord {
    pub should_log: bool,
}

impl VetoNotice {
    pub const fn new() -> Self {
        Self {
            text: String::new(),
            visible_until_ms: 0,
            logged: None,
        }
    }

    pub fn record(&mut self, path: &str, text: &str, now_ms: u64) -> VetoRecord {
        text.clone_into(&mut self.text);
        self.visible_until_ms = now_ms.saturating_add(VETO_PROMPT_VISIBLE_MS);
        let key = format!("{path}|{text}");
        let due = match &self.logged {
            Some((last_key, at_ms)) => {
                *last_key != key || now_ms.saturating_sub(*at_ms) >= VETO_LOG_INTERVAL_MS
            }
            None => true,
        };
        if due {
            self.logged = Some((key, now_ms));
        }
        VetoRecord { should_log: due }
    }

    pub fn visible_text(&self, now_ms: u64) -> Option<&str> {
        (now_ms < self.visible_until_ms && !self.text.is_empty()).then_some(self.text.as_str())
    }
}
