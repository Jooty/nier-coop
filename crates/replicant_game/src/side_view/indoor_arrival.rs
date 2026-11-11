use crate::side_view::{INDOOR_ARRIVAL_SETTLE_MS, SideViewDoor, door_with_start_point};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndoorArrivalReading<'a> {
    pub now_ms: u64,
    pub area_loads: u32,
    pub area: Option<&'a str>,
    pub start_point: i32,
    pub in_play_and_free: bool,
    pub in_side_view: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrivalViewFix {
    Nothing,
    RunEntrySteps(&'static SideViewDoor),
    TurnViewOff,
}

/// Judges the side view once per area load, after the player has been free for a moment so the
/// area's own start has had its say: a door place entered without its view gets the entry steps;
/// a view the mod's own area change carried out of a door place is turned off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndoorArrivalWatch {
    seen_loads: Option<u32>,
    load_pending: bool,
    free_since_ms: Option<u64>,
    mod_left_with_the_view_on: bool,
}

impl IndoorArrivalWatch {
    pub const fn new() -> Self {
        Self {
            seen_loads: None,
            load_pending: false,
            free_since_ms: None,
            mod_left_with_the_view_on: false,
        }
    }

    pub fn mod_requested_an_area_change(&mut self, in_side_view: bool) {
        self.mod_left_with_the_view_on = in_side_view;
    }

    pub fn observe(&mut self, reading: &IndoorArrivalReading) -> ArrivalViewFix {
        // The loads counted before the first look are not arrivals this watch saw.
        let seen = self.seen_loads.replace(reading.area_loads);
        if seen.is_some_and(|seen| seen != reading.area_loads) {
            self.load_pending = true;
            self.free_since_ms = None;
        }
        if !self.load_pending {
            return ArrivalViewFix::Nothing;
        }
        if !reading.in_play_and_free {
            self.free_since_ms = None;
            return ArrivalViewFix::Nothing;
        }
        let free_since_ms = *self.free_since_ms.get_or_insert(reading.now_ms);
        if reading.now_ms.saturating_sub(free_since_ms) < INDOOR_ARRIVAL_SETTLE_MS {
            return ArrivalViewFix::Nothing;
        }
        self.load_pending = false;
        self.free_since_ms = None;
        let carried_by_the_mod = std::mem::take(&mut self.mod_left_with_the_view_on);
        let door_here = reading
            .area
            .and_then(|area| door_with_start_point(area, reading.start_point));
        match (reading.in_side_view, door_here) {
            (false, Some(door)) if !door.entry_steps.is_empty() => {
                ArrivalViewFix::RunEntrySteps(door)
            }
            (true, None) if carried_by_the_mod => ArrivalViewFix::TurnViewOff,
            _ => ArrivalViewFix::Nothing,
        }
    }
}

impl Default for IndoorArrivalWatch {
    fn default() -> Self {
        Self::new()
    }
}
