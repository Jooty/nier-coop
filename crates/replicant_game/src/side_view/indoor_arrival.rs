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

/// Finds the door place an area load put the player into without its side view. It answers once
/// per load, after the player has been free for a moment so the area's own start has had its say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndoorArrivalWatch {
    seen_loads: Option<u32>,
    load_pending: bool,
    free_since_ms: Option<u64>,
}

impl IndoorArrivalWatch {
    pub const fn new() -> Self {
        Self {
            seen_loads: None,
            load_pending: false,
            free_since_ms: None,
        }
    }

    pub fn observe(&mut self, reading: &IndoorArrivalReading) -> Option<&'static SideViewDoor> {
        // The loads counted before the first look are not arrivals this watch saw.
        let seen = self.seen_loads.replace(reading.area_loads);
        if seen.is_some_and(|seen| seen != reading.area_loads) {
            self.load_pending = true;
            self.free_since_ms = None;
        }
        if !self.load_pending {
            return None;
        }
        if !reading.in_play_and_free {
            self.free_since_ms = None;
            return None;
        }
        let free_since_ms = *self.free_since_ms.get_or_insert(reading.now_ms);
        if reading.now_ms.saturating_sub(free_since_ms) < INDOOR_ARRIVAL_SETTLE_MS {
            return None;
        }
        self.load_pending = false;
        self.free_since_ms = None;
        if reading.in_side_view {
            return None;
        }
        door_with_start_point(reading.area?, reading.start_point)
            .filter(|door| !door.entry_steps.is_empty())
    }
}

impl Default for IndoorArrivalWatch {
    fn default() -> Self {
        Self::new()
    }
}
