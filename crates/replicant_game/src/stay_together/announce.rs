use crate::stay_together::{ANNOUNCE_RESEND_MS, FollowRole, PhaseTarget};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnnounceConditions {
    pub role: FollowRole,
    pub connected: bool,
    pub current: Option<PhaseTarget>,
    pub now_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HostAnnouncer {
    last_sent: Option<(PhaseTarget, u64)>,
}

impl HostAnnouncer {
    pub const fn new() -> Self {
        Self { last_sent: None }
    }

    pub fn poll(&mut self, conditions: &AnnounceConditions) -> Option<PhaseTarget> {
        if conditions.role != FollowRole::Host || !conditions.connected {
            self.last_sent = None;
            return None;
        }
        let current = conditions.current?;
        let due = match self.last_sent {
            None => true,
            Some((sent, _)) if sent.phase != current.phase => true,
            Some((_, sent_at)) => conditions.now_ms.saturating_sub(sent_at) >= ANNOUNCE_RESEND_MS,
        };
        if !due {
            return None;
        }
        self.last_sent = Some((current, conditions.now_ms));
        Some(current)
    }
}
