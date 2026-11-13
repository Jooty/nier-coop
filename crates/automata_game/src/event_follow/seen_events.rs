use crate::event_follow::{EVENT_SEEN_REMEMBERED_MS, EventId, MOST_SEEN_EVENTS_REMEMBERED};
use crate::load_gate::StoryPlace;

/// Events the mod started on this copy by their DataId, per story place.
#[derive(Debug, Clone, Default)]
pub struct SeenEvents {
    seen: Vec<(EventId, StoryPlace, u64)>,
}

impl SeenEvents {
    pub fn remember(&mut self, id: EventId, place: StoryPlace, now_ms: u64) {
        self.seen.retain(|(seen, _, _)| *seen != id);
        if self.seen.len() >= MOST_SEEN_EVENTS_REMEMBERED {
            self.seen.remove(0);
        }
        self.seen.push((id, place, now_ms));
    }

    pub fn contains(&self, id: EventId, place: &StoryPlace, now_ms: u64) -> bool {
        self.seen.iter().any(|(seen, seen_place, at_ms)| {
            *seen == id
                && seen_place == place
                && now_ms.saturating_sub(*at_ms) <= EVENT_SEEN_REMEMBERED_MS
        })
    }

    pub fn forget_old(&mut self, now_ms: u64) {
        self.seen
            .retain(|(_, _, at_ms)| now_ms.saturating_sub(*at_ms) <= EVENT_SEEN_REMEMBERED_MS);
    }
}
