const MILLIS_PER_MINUTE: u64 = 60_000;
const BUSY_MINUTE_LINES_PER_KIND: u32 = 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinuteLineBudget {
    lines_per_minute: u32,
    minute: Option<u64>,
    used_this_minute: u32,
    held_back_this_minute: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineAdmission {
    pub allowed: bool,
    pub held_back_note: Option<String>,
}

impl Default for MinuteLineBudget {
    fn default() -> Self {
        Self::new(BUSY_MINUTE_LINES_PER_KIND)
    }
}

impl MinuteLineBudget {
    pub const fn new(lines_per_minute: u32) -> Self {
        Self {
            lines_per_minute,
            minute: None,
            used_this_minute: 0,
            held_back_this_minute: 0,
        }
    }

    pub fn admit(&mut self, now_ms: u64, topic: &str) -> LineAdmission {
        let minute = now_ms / MILLIS_PER_MINUTE;
        let mut held_back_note = None;
        if self.minute != Some(minute) {
            if self.held_back_this_minute > 0 {
                held_back_note = Some(held_back_line(topic, self.held_back_this_minute));
            }
            self.minute = Some(minute);
            self.used_this_minute = 0;
            self.held_back_this_minute = 0;
        }
        let allowed = self.used_this_minute < self.lines_per_minute;
        if allowed {
            self.used_this_minute += 1;
        } else {
            self.held_back_this_minute = self.held_back_this_minute.saturating_add(1);
        }
        LineAdmission {
            allowed,
            held_back_note,
        }
    }

    pub fn lines_for(&mut self, now_ms: u64, topic: &str, line: String) -> Vec<String> {
        let admission = self.admit(now_ms, topic);
        let mut lines = Vec::new();
        lines.extend(admission.held_back_note);
        if admission.allowed {
            lines.push(line);
        }
        lines
    }
}

pub fn held_back_line(topic: &str, held_back: u32) -> String {
    format!("{topic}: {held_back} more lines of this kind held back until the next minute")
}
