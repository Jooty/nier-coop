use crate::one_sided::{BOTH_RAN_WINDOW_MS, MAX_PENDING_WATCHED_CALLBACKS};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Here,
    Partner,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    BothRan {
        name: String,
    },
    OneSided {
        name: String,
        phase: u16,
        ran_on: Side,
    },
}

#[derive(Debug)]
struct Pending {
    name: String,
    phase: u16,
    side: Side,
    at_ms: u64,
}

#[derive(Debug, Default)]
pub struct OneSidedTracker {
    pending: Vec<Pending>,
}

impl OneSidedTracker {
    pub const fn new() -> Self {
        Self {
            pending: Vec::new(),
        }
    }

    pub fn note(&mut self, side: Side, name: &str, phase: u16, now_ms: u64) -> Option<Verdict> {
        let partner_run = self.pending.iter().position(|entry| {
            entry.side != side
                && entry.phase == phase
                && entry.name == name
                && now_ms.saturating_sub(entry.at_ms) <= BOTH_RAN_WINDOW_MS
        });
        if let Some(index) = partner_run {
            self.pending.remove(index);
            return Some(Verdict::BothRan {
                name: name.to_owned(),
            });
        }
        if self.pending.len() < MAX_PENDING_WATCHED_CALLBACKS {
            self.pending.push(Pending {
                name: name.to_owned(),
                phase,
                side,
                at_ms: now_ms,
            });
        }
        None
    }

    pub fn expire(&mut self, now_ms: u64) -> Vec<Verdict> {
        let mut expired = Vec::new();
        self.pending.retain(|entry| {
            if now_ms.saturating_sub(entry.at_ms) <= BOTH_RAN_WINDOW_MS {
                return true;
            }
            expired.push(Verdict::OneSided {
                name: entry.name.clone(),
                phase: entry.phase,
                ran_on: entry.side,
            });
            false
        });
        expired
    }

    pub fn clear(&mut self) {
        self.pending.clear();
    }

    pub fn waiting(&self) -> usize {
        self.pending.len()
    }
}
