use std::collections::BTreeSet;

use crate::fight_doors::FIGHT_DOOR_PARTNER_RADIUS_UNITS;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HeldLocks {
    doors: BTreeSet<u16>,
}

impl HeldLocks {
    pub fn holds_a_lock_near(distance_to_door: Option<f32>) -> bool {
        distance_to_door.is_some_and(|distance| distance <= FIGHT_DOOR_PARTNER_RADIUS_UNITS)
    }

    pub fn hold(&mut self, layout_id: u16) {
        self.doors.insert(layout_id);
    }

    pub fn drop_hold(&mut self, layout_id: u16) -> bool {
        self.doors.remove(&layout_id)
    }

    pub fn is_holding(&self, layout_id: u16) -> bool {
        self.doors.contains(&layout_id)
    }

    pub fn held_ids(&self) -> Vec<u16> {
        self.doors.iter().copied().collect()
    }

    pub fn release_those_left_behind(
        &mut self,
        distance_to_door: impl Fn(u16) -> Option<f32>,
    ) -> Vec<u16> {
        let left: Vec<u16> = self
            .doors
            .iter()
            .copied()
            .filter(|&id| !Self::holds_a_lock_near(distance_to_door(id)))
            .collect();
        for id in &left {
            self.doors.remove(id);
        }
        left
    }

    pub fn clear(&mut self) {
        self.doors.clear();
    }
}
