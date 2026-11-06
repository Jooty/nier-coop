use std::collections::BTreeSet;

use crate::door_sync::{GUEST_USE_WINDOW_MS, MAX_REGISTERED_DOORS};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DoorRegistry {
    ids: BTreeSet<u16>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuestUseWindow {
    uses: Vec<(u16, u64)>,
}

impl DoorRegistry {
    pub fn note(&mut self, layout_id: u16) -> bool {
        self.ids.len() < MAX_REGISTERED_DOORS && self.ids.insert(layout_id)
    }

    pub fn clear(&mut self) {
        self.ids.clear();
    }

    pub fn ids(&self) -> Vec<u16> {
        self.ids.iter().copied().collect()
    }
}

impl GuestUseWindow {
    pub fn note_use(&mut self, layout_id: u16, now_ms: u64) {
        self.uses
            .retain(|&(id, at)| id != layout_id && now_ms.saturating_sub(at) < GUEST_USE_WINDOW_MS);
        self.uses.push((layout_id, now_ms));
    }

    pub fn was_used_recently(&self, layout_id: u16, now_ms: u64) -> bool {
        self.uses
            .iter()
            .any(|&(id, at)| id == layout_id && now_ms.saturating_sub(at) < GUEST_USE_WINDOW_MS)
    }

    pub fn clear(&mut self) {
        self.uses.clear();
    }
}
