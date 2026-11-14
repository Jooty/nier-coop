use crate::event_follow::{
    EVENT_PLAYED_JUST_BEFORE_A_PLACE_CHANGE_COUNTS_FOR_THE_NEW_PLACE_MS, EventId,
    MOST_EVENTS_PLAYED_IN_A_PLACE_REMEMBERED,
};
use crate::load_gate::StoryPlace;

/// Every event this copy played in its current story place, however it began; forgotten when the
/// place changes, except one that started just before the change.
#[derive(Debug, Clone, Default)]
pub struct PlayedHere {
    place: StoryPlace,
    played: Vec<(EventId, u64)>,
}

impl PlayedHere {
    pub fn follow_place(&mut self, place: StoryPlace, now_ms: u64) {
        if place != self.place {
            self.place = place;
            self.played.retain(|(_, started_at_ms)| {
                now_ms.saturating_sub(*started_at_ms)
                    <= EVENT_PLAYED_JUST_BEFORE_A_PLACE_CHANGE_COUNTS_FOR_THE_NEW_PLACE_MS
            });
        }
    }

    pub fn note(&mut self, id: EventId, now_ms: u64) {
        self.played.retain(|(played, _)| *played != id);
        if self.played.len() >= MOST_EVENTS_PLAYED_IN_A_PLACE_REMEMBERED {
            self.played.remove(0);
        }
        self.played.push((id, now_ms));
    }

    pub fn contains(&self, id: EventId) -> bool {
        self.played.iter().any(|(played, _)| *played == id)
    }
}
