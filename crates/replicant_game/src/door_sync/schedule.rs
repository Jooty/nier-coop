use crate::door_sync::SNAPSHOT_SETTLE_MS;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SnapshotSchedule {
    together_in: Option<u16>,
    due_at_ms: Option<u64>,
}

impl SnapshotSchedule {
    pub const fn new() -> Self {
        Self {
            together_in: None,
            due_at_ms: None,
        }
    }

    pub fn observe(&mut self, host_phase: Option<u16>, partner_phase: Option<u16>, now_ms: u64) {
        let together_in = host_phase.filter(|phase| partner_phase == Some(*phase));
        if together_in == self.together_in {
            return;
        }
        self.together_in = together_in;
        self.due_at_ms = together_in.map(|_| now_ms + SNAPSHOT_SETTLE_MS);
    }

    pub fn take_due(&mut self, now_ms: u64) -> Option<u16> {
        let together_in = self.together_in?;
        if self.due_at_ms.is_some_and(|at| now_ms >= at) {
            self.due_at_ms = None;
            return Some(together_in);
        }
        None
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}
