use crate::stay_together::{FOLLOW_RETRY_MS, TOGETHER_FREE_TO_ACT_GRACE_MS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhaseTarget {
    pub phase: u16,
    pub point: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowRole {
    Host,
    Guest,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FollowConditions {
    pub enabled: bool,
    pub role: FollowRole,
    pub connected: bool,
    pub free_to_act: bool,
    pub local_phase: Option<u16>,
    pub now_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitReason {
    PhaseUnreadable,
    NotFreeToAct,
    RetryCooldown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowStep {
    Idle,
    Waiting(WaitReason),
    Go(PhaseTarget),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FollowState {
    pending: Option<PhaseTarget>,
    retry_at_ms: u64,
    first_seen_ms: Option<u64>,
}

impl FollowRole {
    pub fn from_slot(slot: Option<u8>) -> Self {
        match slot {
            Some(0) => Self::Host,
            Some(_) => Self::Guest,
            None => Self::Unknown,
        }
    }
}

impl WaitReason {
    pub fn label(self) -> &'static str {
        match self {
            Self::PhaseUnreadable => "your map is not readable yet (loading?)",
            Self::NotFreeToAct => "waiting until you are free to act",
            Self::RetryCooldown => "waiting for the map change to take",
        }
    }
}

impl FollowState {
    pub const fn new() -> Self {
        Self {
            pending: None,
            retry_at_ms: 0,
            first_seen_ms: None,
        }
    }

    pub fn host_moved_to(&mut self, target: PhaseTarget) {
        if self.pending != Some(target) {
            self.pending = Some(target);
            self.retry_at_ms = 0;
            self.first_seen_ms = None;
        }
    }

    pub fn change_queued_by_rejoin(&mut self, target: PhaseTarget, now_ms: u64) {
        self.pending = Some(target);
        self.first_seen_ms = None;
        self.retry_at_ms = now_ms + FOLLOW_RETRY_MS;
    }

    pub fn pending(&self) -> Option<PhaseTarget> {
        self.pending
    }

    pub fn step(&mut self, conditions: &FollowConditions) -> FollowStep {
        let following =
            conditions.enabled && conditions.role == FollowRole::Guest && conditions.connected;
        let Some(target) = self.pending.filter(|_| following) else {
            self.pending = None;
            self.first_seen_ms = None;
            return FollowStep::Idle;
        };
        let Some(local_phase) = conditions.local_phase else {
            return FollowStep::Waiting(WaitReason::PhaseUnreadable);
        };
        if local_phase == target.phase {
            self.pending = None;
            self.first_seen_ms = None;
            return FollowStep::Idle;
        }
        let first_seen_ms = *self.first_seen_ms.get_or_insert(conditions.now_ms);
        let grace_over =
            conditions.now_ms.saturating_sub(first_seen_ms) >= TOGETHER_FREE_TO_ACT_GRACE_MS;
        if !conditions.free_to_act && !grace_over {
            return FollowStep::Waiting(WaitReason::NotFreeToAct);
        }
        if conditions.now_ms < self.retry_at_ms {
            return FollowStep::Waiting(WaitReason::RetryCooldown);
        }
        self.retry_at_ms = conditions.now_ms + FOLLOW_RETRY_MS;
        FollowStep::Go(target)
    }
}
