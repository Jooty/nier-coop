use replicant_proto::DoorState;

use crate::door_sync::{SNAPSHOT_APPLY_GIVE_UP_MS, SNAPSHOT_APPLY_RETRY_MS};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PendingDoorStates {
    phase: Option<u16>,
    waiting: Vec<DoorState>,
    started_ms: u64,
    last_try_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RetryOutcome {
    pub applied: usize,
    pub given_up: usize,
}

impl PendingDoorStates {
    pub fn receive(&mut self, phase: u16, doors: &[DoorState], now_ms: u64) {
        if self.phase != Some(phase) || self.waiting.is_empty() {
            self.waiting.clear();
            self.started_ms = now_ms;
        }
        self.phase = Some(phase);
        for door in doors {
            self.waiting
                .retain(|waiting| waiting.layout_id != door.layout_id);
            self.waiting.push(*door);
        }
        self.last_try_ms = None;
    }

    pub fn is_empty(&self) -> bool {
        self.waiting.is_empty()
    }

    pub fn retry(
        &mut self,
        local_phase: Option<u16>,
        now_ms: u64,
        mut apply: impl FnMut(&DoorState) -> bool,
    ) -> RetryOutcome {
        let mut outcome = RetryOutcome::default();
        if self.waiting.is_empty()
            || self
                .last_try_ms
                .is_some_and(|at| now_ms.saturating_sub(at) < SNAPSHOT_APPLY_RETRY_MS)
        {
            return outcome;
        }
        self.last_try_ms = Some(now_ms);
        let expired = now_ms.saturating_sub(self.started_ms) >= SNAPSHOT_APPLY_GIVE_UP_MS;
        if local_phase == self.phase {
            let before = self.waiting.len();
            self.waiting.retain(|door| !apply(door));
            outcome.applied = before - self.waiting.len();
        }
        if expired {
            outcome.given_up = self.waiting.len();
            self.waiting.clear();
        }
        outcome
    }
}
