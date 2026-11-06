use crate::new_game::{FIRST_SAVE_GIVE_UP_MS, FIRST_SAVE_PHASE, FIRST_SAVE_SETTLE_MS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstSaveStep {
    Wait,
    Save,
    GiveUp,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirstSaveInputs<'a> {
    pub in_play: bool,
    pub phase: &'a str,
    pub save_manager_busy: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FirstSaveGate {
    armed_at_ms: Option<u64>,
    ready_since_ms: Option<u64>,
    finished: bool,
}

impl FirstSaveGate {
    pub const fn new() -> Self {
        Self {
            armed_at_ms: None,
            ready_since_ms: None,
            finished: false,
        }
    }

    pub fn arm(&mut self, now_ms: u64) {
        if self.armed_at_ms.is_none() {
            self.armed_at_ms = Some(now_ms);
        }
    }

    pub fn is_armed(&self) -> bool {
        self.armed_at_ms.is_some() && !self.finished
    }

    pub fn step(&mut self, inputs: FirstSaveInputs<'_>, now_ms: u64) -> FirstSaveStep {
        let Some(armed_at) = self.armed_at_ms else {
            return FirstSaveStep::Wait;
        };
        if self.finished {
            return FirstSaveStep::Done;
        }
        let ready = inputs.in_play && inputs.phase == FIRST_SAVE_PHASE && !inputs.save_manager_busy;
        if !ready {
            self.ready_since_ms = None;
            if now_ms.saturating_sub(armed_at) >= FIRST_SAVE_GIVE_UP_MS {
                self.finished = true;
                return FirstSaveStep::GiveUp;
            }
            return FirstSaveStep::Wait;
        }
        let ready_since = *self.ready_since_ms.get_or_insert(now_ms);
        if now_ms.saturating_sub(ready_since) < FIRST_SAVE_SETTLE_MS {
            return FirstSaveStep::Wait;
        }
        self.finished = true;
        FirstSaveStep::Save
    }
}
