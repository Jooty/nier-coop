use crate::event_follow::EventId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PauseView<'a> {
    pub rule_active: bool,
    pub event: Option<EventId>,
    pub local_open: bool,
    pub heard: &'a [(EventId, bool)],
    pub now_ms: u64,
}
