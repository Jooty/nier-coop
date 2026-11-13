use crate::event_follow::{EventAsk, EventCall, EventId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventSend {
    Started(EventId),
    Ended { id: EventId, skipped: bool },
    Ask(EventAsk),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EventStep {
    pub sends: Vec<EventSend>,
    pub calls: Vec<EventCall>,
    pub logs: Vec<String>,
    pub playing_story_event: Option<EventId>,
}
