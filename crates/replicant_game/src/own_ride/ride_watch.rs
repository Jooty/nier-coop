use crate::own_ride::{RIDE_END_MOVES_THE_PLAYER_FOR_MS, RIDE_ENDED_BY, RIDE_STARTED_BY};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RideChange {
    Started,
    /// `ridden_ms` is none when this game never saw the ride start.
    Ended {
        by: &'static str,
        ridden_ms: Option<u64>,
    },
}

/// Whether this player rides now, from the ride functions this game runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RideWatch {
    phase: Option<u16>,
    started_ms: Option<u64>,
    ended_ms: Option<u64>,
}

impl RideWatch {
    pub const fn new() -> Self {
        Self {
            phase: None,
            started_ms: None,
            ended_ms: None,
        }
    }

    pub fn enter_phase(&mut self, phase: Option<u16>) {
        if self.phase != phase {
            *self = Self {
                phase,
                ..Self::new()
            };
        }
    }

    pub fn ran(&mut self, function: &str, now_ms: u64) -> Option<RideChange> {
        if function == RIDE_STARTED_BY {
            if self.started_ms.is_some() {
                return None;
            }
            self.started_ms = Some(now_ms);
            self.ended_ms = None;
            return Some(RideChange::Started);
        }
        let by = RIDE_ENDED_BY.iter().find(|end| **end == function)?;
        let ridden_ms = self
            .started_ms
            .take()
            .map(|started| now_ms.saturating_sub(started));
        self.ended_ms = Some(now_ms);
        Some(RideChange::Ended { by, ridden_ms })
    }

    pub const fn is_riding(&self) -> bool {
        self.started_ms.is_some()
    }

    /// The ride moves this player: while it runs, and for a while after its end, which puts the
    /// player at the stop.
    pub fn moves_the_player(&self, now_ms: u64) -> bool {
        self.is_riding()
            || self.ended_ms.is_some_and(|ended| {
                now_ms.saturating_sub(ended) < RIDE_END_MOVES_THE_PLAYER_FOR_MS
            })
    }
}
