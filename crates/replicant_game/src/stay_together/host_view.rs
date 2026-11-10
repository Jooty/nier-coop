use crate::Position;
use crate::stay_together::PhaseTarget;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HostViewFacts {
    pub in_side_view: bool,
    pub last_normal_spot: Option<Position>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HostViewReading {
    pub phase: Option<u16>,
    pub point: i32,
    pub side_view: bool,
    pub in_play: bool,
    pub free_to_act: bool,
    pub position: Option<Position>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct HostViewTracker {
    arrival: Option<(u16, i32)>,
    settled: bool,
    last_normal_phase: Option<u16>,
    last_normal_spot: Option<Position>,
    scripted_interior_points: Vec<(u16, i32)>,
}

impl HostViewTracker {
    pub const fn new() -> Self {
        Self {
            arrival: None,
            settled: false,
            last_normal_phase: None,
            last_normal_spot: None,
            scripted_interior_points: Vec::new(),
        }
    }

    pub fn observe(&mut self, reading: &HostViewReading) -> Option<i32> {
        let Some(phase) = reading.phase else {
            self.arrival = None;
            self.settled = false;
            return None;
        };
        if self.last_normal_phase != Some(phase) {
            self.last_normal_phase = Some(phase);
            self.last_normal_spot = None;
        }
        if self.arrival != Some((phase, reading.point)) {
            self.arrival = Some((phase, reading.point));
            self.settled = false;
        }
        if reading.in_play
            && !reading.side_view
            && let Some(position) = reading.position
        {
            self.last_normal_spot = Some(position);
        }
        if self.settled || !(reading.in_play && reading.free_to_act) {
            return None;
        }
        self.settled = true;
        let marks_new_point = reading.side_view
            && reading.point != 0
            && !self
                .scripted_interior_points
                .contains(&(phase, reading.point));
        if !marks_new_point {
            return None;
        }
        self.scripted_interior_points.push((phase, reading.point));
        Some(reading.point)
    }

    pub fn announced_point(&self, target: PhaseTarget) -> i32 {
        if self
            .scripted_interior_points
            .contains(&(target.phase, target.point))
        {
            0
        } else {
            target.point
        }
    }

    pub fn facts(&self, in_side_view: bool) -> HostViewFacts {
        HostViewFacts {
            in_side_view,
            last_normal_spot: self.last_normal_spot,
        }
    }
}
