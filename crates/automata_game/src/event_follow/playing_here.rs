use crate::event_follow::{EventId, StartOrigin};

/// An event that started on this copy and has not ended yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayingHere {
    pub id: EventId,
    pub since_ms: u64,
    pub origin: StartOrigin,
    pub told: bool,
}

pub fn story_event_playing(playing: &[PlayingHere], id: EventId) -> bool {
    playing
        .iter()
        .any(|here| here.id == id && here.origin.is_a_story_event())
}
