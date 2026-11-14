use crate::load_gate::StoryPlace;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotChange {
    pub body_handle: u32,
    pub slot_before: u32,
    pub slot_after: u32,
    pub place_before: StoryPlace,
    pub place_now: StoryPlace,
    pub since_place_change_ms: Option<u64>,
}

/// Watches the hero's partner slot and the story place, and tells when a watched body left the slot.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SlotWatch {
    last_slot: Option<u32>,
    place: StoryPlace,
    place_before: StoryPlace,
    place_changed_ms: Option<u64>,
}

impl SlotWatch {
    /// `slot` is None when there is no hero to read it from; `watched` is the body this copy holds or
    /// gave back to the game.
    pub fn observe(
        &mut self,
        now_ms: u64,
        slot: Option<u32>,
        watched: Option<u32>,
        place: StoryPlace,
    ) -> Option<SlotChange> {
        if place != self.place {
            self.place_before = self.place;
            self.place = place;
            self.place_changed_ms = Some(now_ms);
        }
        let last_slot = std::mem::replace(&mut self.last_slot, slot);
        let (Some(slot_before), Some(slot_after), Some(body_handle)) = (last_slot, slot, watched)
        else {
            return None;
        };
        (slot_before == body_handle && slot_after != body_handle).then(|| SlotChange {
            body_handle,
            slot_before,
            slot_after,
            place_before: self.place_before,
            place_now: self.place,
            since_place_change_ms: self
                .place_changed_ms
                .map(|changed_ms| now_ms.saturating_sub(changed_ms)),
        })
    }
}
