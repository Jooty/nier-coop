use crate::Position;
use crate::stay_together::{
    ARRIVAL_PLACEMENT_RETRY_EVERY_MS, ARRIVAL_PLACEMENT_TRIES, HostViewFacts,
    TELEPORT_ARRIVED_UNITS, distance_between,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlacementVerdict {
    BesideHost,
    OutsideSpot(Position),
    StayAtEntry,
    StayWhereYouAre,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadBack {
    Placed,
    Waiting,
    NotTaken { try_number: u32 },
    GaveUp { tries: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlacementCheck {
    target: Position,
    tries: u32,
    last_request_ms: u64,
}

impl PlacementCheck {
    pub const fn requested(target: Position, now_ms: u64) -> Self {
        Self {
            target,
            tries: 1,
            last_request_ms: now_ms,
        }
    }

    pub fn target(&self) -> Position {
        self.target
    }

    pub fn tries(&self) -> u32 {
        self.tries
    }

    pub fn read_back(&mut self, now: Position, now_ms: u64) -> ReadBack {
        let near = distance_between(
            [self.target.x, self.target.y, self.target.z],
            [now.x, now.y, now.z],
        )
        .is_some_and(|gap| gap <= TELEPORT_ARRIVED_UNITS);
        if near {
            return ReadBack::Placed;
        }
        if now_ms.saturating_sub(self.last_request_ms) < ARRIVAL_PLACEMENT_RETRY_EVERY_MS {
            return ReadBack::Waiting;
        }
        if self.tries >= ARRIVAL_PLACEMENT_TRIES {
            return ReadBack::GaveUp { tries: self.tries };
        }
        let failed = self.tries;
        self.tries += 1;
        self.last_request_ms = now_ms;
        ReadBack::NotTaken { try_number: failed }
    }
}

pub fn decide_placement(host: Option<HostViewFacts>, guest_in_side_view: bool) -> PlacementVerdict {
    let Some(host) = host else {
        return PlacementVerdict::BesideHost;
    };
    if host.in_side_view == guest_in_side_view {
        return PlacementVerdict::BesideHost;
    }
    if guest_in_side_view {
        return PlacementVerdict::StayWhereYouAre;
    }
    match host.last_normal_spot {
        Some(spot) => PlacementVerdict::OutsideSpot(spot),
        None => PlacementVerdict::StayAtEntry,
    }
}
