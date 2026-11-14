use crate::event_follow::EventId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventProgress {
    pub id: EventId,
    pub frame: i32,
    pub last_frame: i32,
    pub paused: bool,
}
