use crate::event_follow::{
    EventAsk, EventId, EventSlot, PartnerReport, STORY_EVENTS_BELOW_PHASE_ID, Unattended,
};
use crate::load_gate::{PlaceRelation, StoryPlace};
use crate::travel::JumpRole;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventReadings {
    pub now_ms: u64,
    pub role: JumpRole,
    pub partner_present: bool,
    pub local_in_play: bool,
    pub partner_in_play: bool,
    pub current_phase_id: i32,
    pub current_place: StoryPlace,
    pub partner_place: StoryPlace,
    /// The partner's playing story event from its latest fresh state, if any.
    pub partner_playing: Option<EventId>,
    pub slots: Vec<EventSlot>,
    pub event_flags_say_playing: bool,
    pub started_heard: Vec<EventId>,
    pub ended_heard: Vec<(EventId, bool)>,
    pub asks: Vec<EventAsk>,
    pub partner_report: Option<PartnerReport>,
    pub local_unattended: Option<Unattended>,
    pub skip_asks_heard: Vec<EventId>,
}

impl EventReadings {
    pub fn linked_role(&self) -> JumpRole {
        if self.partner_present {
            self.role
        } else {
            JumpRole::Solo
        }
    }

    pub fn rule_role(&self) -> JumpRole {
        let both_in_play = self.local_in_play && self.partner_in_play;
        let story_phase = (0..STORY_EVENTS_BELOW_PHASE_ID).contains(&self.current_phase_id);
        if both_in_play && story_phase {
            self.linked_role()
        } else {
            JumpRole::Solo
        }
    }

    pub fn plays_an_event(&self) -> bool {
        self.event_flags_say_playing || self.slots.iter().any(|slot| slot.is_playing())
    }

    pub fn in_the_partners_place(&self) -> bool {
        self.current_place.relation_to(&self.partner_place) == PlaceRelation::Same
    }

    pub fn slot_of(&self, id: EventId) -> Option<EventSlot> {
        self.slots.iter().copied().find(|slot| slot.id == id)
    }
}
