use std::collections::VecDeque;

use rc_proto::StoryCall;

use crate::fight_sync::SlotState;
use crate::story_relay::{MAX_WAITING_RELAY_CALLS, RELAY_CALL_POP_EXPECTED_MS};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaitingRelayCall<T> {
    pub call: StoryCall,
    pub then: T,
    pub submitted_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayCallSubmitted {
    Waiting { position: usize },
    Full,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelayCallFlight {
    Idle,
    Queued {
        name: String,
        slot: u64,
        since_ms: u64,
    },
    Running {
        name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelayCallNotice {
    QueuedCallNeverPopped { name: String, waited_ms: u64 },
}

/// The one gate every relay path (callback follow, end queue, message handlers, shared functions
/// and fill-ins, scene follow, story events) queues the partner's calls through. The engine reuses
/// the first free queue slot, so only one of these calls is in the engine queue at a time: the
/// next is handed out only once the previous popped and the script slot went idle again, in the
/// order the paths submitted them.
#[derive(Debug, Clone)]
pub struct RelayCallGate<T> {
    waiting: VecDeque<WaitingRelayCall<T>>,
    in_flight: RelayCallFlight,
    notices: Vec<RelayCallNotice>,
}

impl<T> Default for RelayCallGate<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> RelayCallGate<T> {
    pub const fn new() -> Self {
        Self {
            waiting: VecDeque::new(),
            in_flight: RelayCallFlight::Idle,
            notices: Vec::new(),
        }
    }

    pub fn submit(&mut self, call: StoryCall, then: T, now_ms: u64) -> RelayCallSubmitted {
        if self.waiting.len() >= MAX_WAITING_RELAY_CALLS {
            return RelayCallSubmitted::Full;
        }
        self.waiting.push_back(WaitingRelayCall {
            call,
            then,
            submitted_ms: now_ms,
        });
        RelayCallSubmitted::Waiting {
            position: self.waiting.len(),
        }
    }

    /// Called once per tick with the script slot's state; returns the call whose turn came.
    /// Nothing goes while another queue path (the fight stream) has its own call in the engine
    /// queue. A call that shows modal UI may go while stepping is held: it closes the menu.
    pub fn tick(
        &mut self,
        slot: SlotState,
        other_call_queued: bool,
        now_ms: u64,
        shows_modal_ui: impl Fn(&StoryCall) -> bool,
    ) -> Option<WaitingRelayCall<T>> {
        match &self.in_flight {
            RelayCallFlight::Running { .. } if slot == SlotState::Idle => {
                self.in_flight = RelayCallFlight::Idle;
            }
            RelayCallFlight::Queued { name, since_ms, .. }
                if now_ms.saturating_sub(*since_ms) > RELAY_CALL_POP_EXPECTED_MS =>
            {
                self.notices.push(RelayCallNotice::QueuedCallNeverPopped {
                    name: name.clone(),
                    waited_ms: now_ms.saturating_sub(*since_ms),
                });
                self.in_flight = RelayCallFlight::Idle;
            }
            RelayCallFlight::Idle
            | RelayCallFlight::Queued { .. }
            | RelayCallFlight::Running { .. } => {}
        }
        if self.in_flight != RelayCallFlight::Idle || other_call_queued {
            return None;
        }
        let front = self.waiting.front()?;
        let may_go = match slot {
            SlotState::Idle => true,
            SlotState::SteppingHeld => shows_modal_ui(&front.call),
            SlotState::Busy => false,
        };
        if !may_go {
            return None;
        }
        self.waiting.pop_front()
    }

    /// The call the last `tick` returned is in the engine queue at `slot` (its argument block).
    pub fn queued(&mut self, name: &str, slot: u64, now_ms: u64) {
        self.in_flight = RelayCallFlight::Queued {
            name: name.to_owned(),
            slot,
            since_ms: now_ms,
        };
    }

    /// The engine popped a queued call; true when it is the one this gate queued, which now runs.
    pub fn popped(&mut self, slot: u64) -> bool {
        let RelayCallFlight::Queued {
            name, slot: queued, ..
        } = &self.in_flight
        else {
            return false;
        };
        if *queued != slot {
            return false;
        }
        self.in_flight = RelayCallFlight::Running { name: name.clone() };
        true
    }

    /// The queued call was taken back out of the engine queue before it popped.
    pub fn cancelled(&mut self, slot: u64) -> bool {
        let is_ours = matches!(
            &self.in_flight,
            RelayCallFlight::Queued { slot: queued, .. } if *queued == slot
        );
        if is_ours {
            self.in_flight = RelayCallFlight::Idle;
        }
        is_ours
    }

    /// This game is in `phase` now: waiting calls of other phases are dropped. Returns how many.
    pub fn enter_phase(&mut self, phase: Option<u16>) -> usize {
        let before = self.waiting.len();
        self.waiting
            .retain(|waiting| Some(waiting.call.phase) == phase);
        before - self.waiting.len()
    }

    /// The engine queue was zeroed (a phase load or restart): the call in flight is gone.
    pub fn forget_in_flight(&mut self) -> Option<String> {
        let name = match std::mem::replace(&mut self.in_flight, RelayCallFlight::Idle) {
            RelayCallFlight::Idle => return None,
            RelayCallFlight::Queued { name, .. } | RelayCallFlight::Running { name } => name,
        };
        Some(name)
    }

    pub fn clear(&mut self) -> usize {
        let dropped = self.waiting.len();
        self.waiting.clear();
        self.in_flight = RelayCallFlight::Idle;
        dropped
    }

    /// Nothing waits and nothing of this gate is in the engine queue or running.
    pub fn is_clear(&self) -> bool {
        self.waiting.is_empty() && self.in_flight == RelayCallFlight::Idle
    }

    pub fn has_call_queued(&self) -> bool {
        matches!(self.in_flight, RelayCallFlight::Queued { .. })
    }

    pub fn waiting_len(&self) -> usize {
        self.waiting.len()
    }

    pub const fn in_flight(&self) -> &RelayCallFlight {
        &self.in_flight
    }

    pub fn take_notices(&mut self) -> Vec<RelayCallNotice> {
        std::mem::take(&mut self.notices)
    }
}
