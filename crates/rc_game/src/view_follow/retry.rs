use crate::view_follow::{
    VIEW_RETRY_CAP_MS, VIEW_RETRY_INTERVAL_MS, VIEW_SCENE_OVER_FALLBACK_MS, VIEW_SCENE_WAIT_CAP_MS,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetryAction {
    Queue(String),
    GiveUp(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RanReport {
    pub name: String,
    pub after_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Stage {
    Waiting,
    Queued { first_ms: u64, last_ms: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Pending {
    name: String,
    stage: Stage,
}

#[derive(Debug, Default)]
pub struct ViewRetry {
    pending: Vec<Pending>,
    requested_ms: u64,
    fade_started: bool,
    fade_finished: bool,
    natural_pop: bool,
}

impl ViewRetry {
    pub const fn new() -> Self {
        Self {
            pending: Vec::new(),
            requested_ms: 0,
            fade_started: false,
            fade_finished: false,
            natural_pop: false,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn is_idle(&self) -> bool {
        self.pending.is_empty()
    }

    pub fn request(&mut self, names: &[&str], now_ms: u64) {
        self.pending = names
            .iter()
            .map(|name| Pending {
                name: (*name).to_owned(),
                stage: Stage::Waiting,
            })
            .collect();
        self.requested_ms = now_ms;
        self.fade_started = false;
        self.fade_finished = false;
        self.natural_pop = false;
    }

    pub fn note_fade(&mut self, started: bool) {
        if self.pending.is_empty() {
            return;
        }
        if started {
            self.fade_started = true;
        } else {
            self.fade_finished = true;
        }
    }

    pub fn note_pop(&mut self, name: &str, now_ms: u64) -> Option<RanReport> {
        if self.pending.is_empty() {
            return None;
        }
        let Some(index) = self.pending.iter().position(|entry| entry.name == name) else {
            self.natural_pop = true;
            return None;
        };
        let entry = self.pending.remove(index);
        let from_ms = match entry.stage {
            Stage::Queued { first_ms, .. } => first_ms,
            Stage::Waiting => self.requested_ms,
        };
        Some(RanReport {
            name: entry.name,
            after_ms: now_ms.saturating_sub(from_ms),
        })
    }

    pub fn poll(&mut self, now_ms: u64) -> Vec<RetryAction> {
        let over = self.scene_is_over(now_ms);
        let mut actions = Vec::new();
        self.pending.retain_mut(|entry| match entry.stage {
            Stage::Waiting => {
                if over {
                    entry.stage = Stage::Queued {
                        first_ms: now_ms,
                        last_ms: now_ms,
                    };
                    actions.push(RetryAction::Queue(entry.name.clone()));
                }
                true
            }
            Stage::Queued { first_ms, last_ms } => {
                if now_ms.saturating_sub(first_ms) >= VIEW_RETRY_CAP_MS {
                    actions.push(RetryAction::GiveUp(entry.name.clone()));
                    false
                } else {
                    if now_ms.saturating_sub(last_ms) >= VIEW_RETRY_INTERVAL_MS {
                        entry.stage = Stage::Queued {
                            first_ms,
                            last_ms: now_ms,
                        };
                        actions.push(RetryAction::Queue(entry.name.clone()));
                    }
                    true
                }
            }
        });
        actions
    }

    fn scene_is_over(&self, now_ms: u64) -> bool {
        let waited = now_ms.saturating_sub(self.requested_ms);
        if waited >= VIEW_SCENE_WAIT_CAP_MS {
            return true;
        }
        if !self.natural_pop {
            return false;
        }
        if self.fade_started {
            self.fade_finished
        } else {
            waited >= VIEW_SCENE_OVER_FALLBACK_MS
        }
    }
}
