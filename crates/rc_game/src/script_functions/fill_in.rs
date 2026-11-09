use std::collections::VecDeque;

use crate::fight_sync::SlotState;
use crate::script_functions::{
    FILL_IN_GRACE_MS, FILL_IN_QUEUED_CALL_POP_EXPECTED_MS, is_called_only_from_local_state,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillInReach {
    /// This game's own engine or map init runs its own copy, so the partner's copy waits for it.
    OwnCopyExpected,
    /// Called only from functions that hold local state (a hit counter, a life check): this game
    /// may never run it, so the partner's copy runs as soon as its turn comes.
    CannotReachHere,
}

pub fn fill_in_reach(phase_label: &str, name: &str) -> FillInReach {
    if is_called_only_from_local_state(phase_label, name) {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FillInFlight {
    Idle,
    Queued { name: String, since_ms: u64 },
    Running { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FillInNotice {
    QueuedCallNeverRan { name: String, waited_ms: u64 },
}

/// The partner's copies of shared story steps outside a fight window. This game runs its own
/// copy of each step; the partner's copy runs here only when this game has not run the step
/// itself within `FILL_IN_GRACE_MS`, or at once when this game cannot reach the step on its own.
/// Copies run in arrival order and one at a time: the engine reuses the first free queue slot,
/// so a copy is queued only when the script slot is idle and the previous copy ran and ended.
#[derive(Debug, Clone)]
pub struct FillInRelay {
    waiting: VecDeque<FillInCopy>,
    in_flight: FillInFlight,
    notices: Vec<FillInNotice>,
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
            in_flight: FillInFlight::Idle,
            notices: Vec::new(),
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

    /// Called once per tick with the script slot's state; returns the copy whose turn came. A copy
    /// that shows modal UI may go while stepping is held (a menu is open): it closes the menu.
    pub fn tick(
        &mut self,
        slot: SlotState,
        now_ms: u64,
        shows_modal_ui: impl Fn(&FillInCopy) -> bool,
    ) -> Option<FillInCopy> {
        match &self.in_flight {
            FillInFlight::Running { .. } if slot == SlotState::Idle => {
                self.in_flight = FillInFlight::Idle;
            }
            FillInFlight::Queued { name, since_ms }
                if now_ms.saturating_sub(*since_ms) > FILL_IN_QUEUED_CALL_POP_EXPECTED_MS =>
            {
                self.notices.push(FillInNotice::QueuedCallNeverRan {
                    name: name.clone(),
                    waited_ms: now_ms.saturating_sub(*since_ms),
                });
                self.in_flight = FillInFlight::Idle;
            }
            FillInFlight::Idle | FillInFlight::Queued { .. } | FillInFlight::Running { .. } => {}
        }
        if self.in_flight != FillInFlight::Idle {
            return None;
        }
        let front = self.waiting.front()?;
        if front.due_ms() > now_ms {
            return None;
        }
        let may_go = match slot {
            SlotState::Idle => true,
            SlotState::SteppingHeld => shows_modal_ui(front),
            SlotState::Busy => false,
        };
        if !may_go {
            return None;
        }
        self.waiting.pop_front()
    }

    /// The copy the last `tick` returned was handed to the engine's queue.
    pub fn queued(&mut self, name: &str, now_ms: u64) {
        self.in_flight = FillInFlight::Queued {
            name: name.to_owned(),
            since_ms: now_ms,
        };
    }

    /// A function started here; true when it is the queued copy, which now runs.
    pub fn started(&mut self, name: &str) -> bool {
        let FillInFlight::Queued { name: queued, .. } = &self.in_flight else {
            return false;
        };
        if queued != name {
            return false;
        }
        self.in_flight = FillInFlight::Running {
            name: name.to_owned(),
        };
        true
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
        self.in_flight = FillInFlight::Idle;
        dropped
    }

    pub fn take_notices(&mut self) -> Vec<FillInNotice> {
        std::mem::take(&mut self.notices)
    }

    pub fn waiting_len(&self) -> usize {
        self.waiting.len()
    }

    pub const fn in_flight(&self) -> &FillInFlight {
        &self.in_flight
    }

    fn position_of(&self, phase: u16, name: &str) -> Option<usize> {
        self.waiting
            .iter()
            .position(|copy| copy.phase == phase && copy.name == name)
    }
}
