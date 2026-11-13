use crate::event_follow::EventId;

/// What the partner's latest state report says for a skip vote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartnerReport {
    pub playing: Option<EventId>,
    pub unattended: bool,
    pub age_ms: u64,
}
