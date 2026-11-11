use crate::zone_rules::VETO_LOG_INTERVAL_MS;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VetoNotice {
    logged: Option<(String, u64)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VetoRecord {
    pub should_log: bool,
}

impl VetoNotice {
    pub const fn new() -> Self {
        Self { logged: None }
    }

    pub fn record(&mut self, path: &str, text: &str, now_ms: u64) -> VetoRecord {
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
}
