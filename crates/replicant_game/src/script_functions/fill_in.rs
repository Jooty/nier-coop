use std::collections::VecDeque;

use crate::script_functions::{FILL_IN_GRACE_MS, is_called_only_from_local_state};
use crate::shared_sequence::is_led_by_the_host;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillInReach {
    /// This game's own engine or map init runs its own copy, so the partner's copy waits for it.
    OwnCopyExpected,
    /// Called only from functions that hold local state (a hit counter, a life check), or a step
    /// only the host's guide reaches: this game may never run it, so the partner's copy runs as
    /// soon as its turn comes.
    CannotReachHere,
}

pub fn fill_in_reach(phase_label: &str, name: &str) -> FillInReach {
    if is_called_only_from_local_state(phase_label, name) || is_led_by_the_host(phase_label, name) {
        FillInReach::CannotReachHere
    } else {
        FillInReach::OwnCopyExpected
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FillInCopy {
    pub phase: u16,
    pub name: String,
    pub reach: FillInReach,
    pub arrived_ms: u64,
}

impl FillInCopy {
    pub const fn due_ms(&self) -> u64 {
        match self.reach {
            FillInReach::OwnCopyExpected => self.arrived_ms.saturating_add(FILL_IN_GRACE_MS),
            FillInReach::CannotReachHere => self.arrived_ms,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillInArrival {
    Waiting { position: usize, due_in_ms: u64 },
    AlreadyWaiting,
}

/// The partner's copies of shared story steps outside a fight window. This game runs its own
/// copy of each step; the partner's copy runs here only when this game has not run the step
/// itself within `FILL_IN_GRACE_MS`, or at once when this game cannot reach the step on its own.
/// Copies go in arrival order, each only once the shared relay call gate is clear, so a copy
/// that is dropped when its turn comes never holds up the next one.
#[derive(Debug, Clone)]
pub struct FillInRelay {
    waiting: VecDeque<FillInCopy>,
}

impl Default for FillInRelay {
    fn default() -> Self {
        Self::new()
    }
}

impl FillInRelay {
    pub const fn new() -> Self {
        Self {
            waiting: VecDeque::new(),
        }
    }

    pub fn arrive(
        &mut self,
        phase: u16,
        name: &str,
        reach: FillInReach,
        now_ms: u64,
    ) -> FillInArrival {
        if self.position_of(phase, name).is_some() {
            return FillInArrival::AlreadyWaiting;
        }
        let copy = FillInCopy {
            phase,
            name: name.to_owned(),
            reach,
            arrived_ms: now_ms,
        };
        let due_in_ms = copy.due_ms().saturating_sub(now_ms);
        self.waiting.push_back(copy);
        FillInArrival::Waiting {
            position: self.waiting.len(),
            due_in_ms,
        }
    }

    /// This game ran `name` itself: the partner's waiting copy of it is no longer needed.
    pub fn ran_here(&mut self, phase: u16, name: &str) -> Option<FillInCopy> {
        let index = self.position_of(phase, name)?;
        self.waiting.remove(index)
    }

    /// Called once per tick; returns the copy whose turn came. `relay_calls_clear` is true when
    /// the shared relay call gate has nothing waiting and nothing in the engine queue.
    pub fn tick(&mut self, relay_calls_clear: bool, now_ms: u64) -> Option<FillInCopy> {
        if !relay_calls_clear {
            return None;
        }
        let front = self.waiting.front()?;
        if front.due_ms() > now_ms {
            return None;
        }
        self.waiting.pop_front()
    }

    /// This game is in `phase` now: copies of other phases are dropped. Returns how many.
    pub fn enter_phase(&mut self, phase: Option<u16>) -> usize {
        let before = self.waiting.len();
        self.waiting.retain(|copy| Some(copy.phase) == phase);
        before - self.waiting.len()
    }

    pub fn clear(&mut self) -> usize {
        let dropped = self.waiting.len();
        self.waiting.clear();
        dropped
    }

    pub fn waiting_len(&self) -> usize {
        self.waiting.len()
    }

    fn position_of(&self, phase: u16, name: &str) -> Option<usize> {
        self.waiting
            .iter()
            .position(|copy| copy.phase == phase && copy.name == name)
    }
}
