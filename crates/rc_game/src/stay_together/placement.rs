use crate::Position;
use crate::stay_together::{
    ARRIVAL_PLACEMENT_TRIES, HostViewFacts, TELEPORT_ARRIVED_UNITS, distance_between,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlacementVerdict {
    BesideHost,
    OutsideSpot(Position),
    StayAtEntry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadBack {
    Placed,
    NotTaken { try_number: u32 },
    GaveUp { tries: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlacementCheck {
    target: Position,
    tries: u32,
}

impl PlacementCheck {
    pub const fn requested(target: Position) -> Self {
        Self { target, tries: 1 }
    }

    pub fn target(&self) -> Position {
        self.target
    }

    pub fn tries(&self) -> u32 {
        self.tries
    }

    pub fn read_back(&mut self, now: Position) -> ReadBack {
        let near = distance_between(
            [self.target.x, self.target.y, self.target.z],
            [now.x, now.y, now.z],
        )
        .is_some_and(|gap| gap <= TELEPORT_ARRIVED_UNITS);
        if near {
            return ReadBack::Placed;
        }
        if self.tries >= ARRIVAL_PLACEMENT_TRIES {
            return ReadBack::GaveUp { tries: self.tries };
        }
        let failed = self.tries;
        self.tries += 1;
        ReadBack::NotTaken { try_number: failed }
    }
}

pub fn decide_placement(host: Option<HostViewFacts>, guest_in_side_view: bool) -> PlacementVerdict {
    let Some(host) = host.filter(|host| host.in_side_view && !guest_in_side_view) else {
        return PlacementVerdict::BesideHost;
    };
    match host.last_normal_spot {
        Some(spot) => PlacementVerdict::OutsideSpot(spot),
        None => PlacementVerdict::StayAtEntry,
    }
}
