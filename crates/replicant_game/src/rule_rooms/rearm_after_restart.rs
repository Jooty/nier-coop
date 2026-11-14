use crate::rule_rooms::{REARM_NOT_BEFORE_RESTART_POP_MS, RuleRoom};

/// A room that resets as one has its breaks and clock armed again once its restart has finished:
/// the pack's own restart arms the wrong room's rule.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RearmAfterRestart {
    waiting: Option<(&'static RuleRoom, u64)>,
}

impl RearmAfterRestart {
    pub const fn new() -> Self {
        Self { waiting: None }
    }

    pub fn restart_popped(&mut self, room: &'static RuleRoom, now_ms: u64) {
        self.waiting = Some((room, now_ms));
    }

    pub fn take_if_restart_finished(
        &mut self,
        now_ms: u64,
        script_slot_idle: bool,
    ) -> Option<&'static RuleRoom> {
        let (room, popped_ms) = self.waiting?;
        let waited = now_ms.saturating_sub(popped_ms);
        if !script_slot_idle || waited < REARM_NOT_BEFORE_RESTART_POP_MS {
            return None;
        }
        self.waiting = None;
        Some(room)
    }
}
