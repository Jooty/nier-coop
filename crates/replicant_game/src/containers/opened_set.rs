use std::collections::BTreeSet;

use crate::containers::MAX_OPENED_PER_VISIT;

#[derive(Debug, Default)]
pub struct OpenedSet {
    phase: Option<u16>,
    action_ids: BTreeSet<u16>,
}

impl OpenedSet {
    pub fn enter(&mut self, phase: Option<u16>) -> bool {
        if self.phase == phase {
            return false;
        }
        self.phase = phase;
        self.action_ids.clear();
        true
    }

    pub fn claim(&mut self, action_id: u16) -> bool {
        if self.action_ids.len() >= MAX_OPENED_PER_VISIT {
            return false;
        }
        self.action_ids.insert(action_id)
    }

    pub fn contains(&self, action_id: u16) -> bool {
        self.action_ids.contains(&action_id)
    }

    pub fn ids(&self) -> Vec<u16> {
        self.action_ids.iter().copied().collect()
    }
}
