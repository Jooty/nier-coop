use crate::story_follow::{
    PENDING_UNEQUAL_TO_CURRENT_TOLERATED_AFTER_MS, RequestReadiness,
    pending_unequal_to_current_tolerated_line,
};

#[derive(Debug, Clone, Default)]
pub(super) struct PendingTolerance {
    unequal_since_ms: Option<u64>,
    tolerated_logged: bool,
}

impl PendingTolerance {
    pub(super) fn can_take_a_request(
        &mut self,
        readiness: &RequestReadiness,
        now_ms: u64,
        logs: &mut Vec<String>,
    ) -> bool {
        if readiness.can_take_a_request() {
            self.unequal_since_ms = None;
            self.tolerated_logged = false;
            return true;
        }
        if !readiness.is_idle_with_pending_unequal_to_current() {
            self.unequal_since_ms = None;
            self.tolerated_logged = false;
            return false;
        }
        let since_ms = *self.unequal_since_ms.get_or_insert(now_ms);
        let differed_ms = now_ms.saturating_sub(since_ms);
        if differed_ms < PENDING_UNEQUAL_TO_CURRENT_TOLERATED_AFTER_MS {
            return false;
        }
        if !self.tolerated_logged {
            self.tolerated_logged = true;
            logs.push(pending_unequal_to_current_tolerated_line(
                differed_ms,
                readiness,
            ));
        }
        true
    }
}
