use std::collections::VecDeque;

use replicant_proto::StoryCall;

use crate::fight_sync::{SlotState, counted_wait_ms};
use crate::story_relay::{
    MAX_WAITING_RELAY_CALLS, RELAY_CALL_LONG_WAIT_LOGGED_EVERY_MS, RELAY_CALL_POP_EXPECTED_MS,
    RELAY_CALL_QUEUE_TRIES, RELAY_CALL_STILL_QUEUED_LOGGED_AFTER_MS,
};

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
    /// `waited_ms` counts only while the script slot was not held (a menu).
    Queued {
        name: String,
        slot: u64,
        waited_ms: u64,
        long_wait_logged: bool,
    },
    Running {
        name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelayCallNotice {
    /// Past the expected wait the slot no longer held the call and it never popped: the gate
    /// is free again.
    QueuedCallNeverPopped { name: String, waited_ms: u64 },
    /// The call is still in the engine queue after this long; the gate keeps waiting for it.
    QueuedCallStillWaiting { name: String, waited_ms: u64 },
    /// The engine queue refused the call this many ticks in a row; it is dropped.
    QueueingGaveUp { name: String, tries: u32 },
}

/// The call at the front of the waiting list has waited `waited_ms` there (menu time not
/// counted), with `waiting` calls in the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LongRelayWait {
    pub front: String,
    pub waited_ms: u64,
    pub waiting: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FrontWait {
    name: String,
    submitted_ms: u64,
    counted_ms: u64,
    last_ms: u64,
    next_report_after_ms: u64,
}

/// What became of a call the engine queue refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueRetry<T> {
    /// Back at the front; the next tick tries again.
    Again {
        tries: u32,
    },
    GaveUp(WaitingRelayCall<T>),
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
    last_check_ms: Option<u64>,
    queue_tries: u32,
    front_wait: Option<FrontWait>,
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
            last_check_ms: None,
            queue_tries: 0,
            front_wait: None,
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
        shows_modal_ui: impl Fn(&StoryCall) -> bool,
    ) -> Option<WaitingRelayCall<T>> {
        if matches!(self.in_flight, RelayCallFlight::Running { .. }) && slot == SlotState::Idle {
            self.in_flight = RelayCallFlight::Idle;
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

    /// Once per tick, before `tick`: `still_queued(name, slot)` reads whether that queue slot still
    /// holds the queued call. Past the expected wait (time with stepping held not counted) a call
    /// whose slot no longer holds it frees the gate; one still there keeps it, logged once past
    /// the long wait, so two of these calls are never in the engine queue at once.
    pub fn check_queued(
        &mut self,
        slot: SlotState,
        now_ms: u64,
        still_queued: impl FnOnce(&str, u64) -> bool,
    ) {
        let counted_ms = counted_wait_ms(slot, self.last_check_ms, now_ms);
        self.last_check_ms = Some(now_ms);
        let RelayCallFlight::Queued {
            name,
            slot: at,
            waited_ms,
            long_wait_logged,
        } = &mut self.in_flight
        else {
            return;
        };
        *waited_ms = waited_ms.saturating_add(counted_ms);
        if *waited_ms <= RELAY_CALL_POP_EXPECTED_MS {
            return;
        }
        if !still_queued(name, *at) {
            self.notices.push(RelayCallNotice::QueuedCallNeverPopped {
                name: name.clone(),
                waited_ms: *waited_ms,
            });
            self.in_flight = RelayCallFlight::Idle;
            return;
        }
        if *waited_ms > RELAY_CALL_STILL_QUEUED_LOGGED_AFTER_MS && !*long_wait_logged {
            *long_wait_logged = true;
            self.notices.push(RelayCallNotice::QueuedCallStillWaiting {
                name: name.clone(),
                waited_ms: *waited_ms,
            });
        }
    }

    /// The call the last `tick` returned is in the engine queue at `slot` (its argument block).
    pub fn queued(&mut self, name: &str, slot: u64) {
        self.queue_tries = 0;
        self.in_flight = RelayCallFlight::Queued {
            name: name.to_owned(),
            slot,
            waited_ms: 0,
            long_wait_logged: false,
        };
    }

    pub const fn queue_tries(&self) -> u32 {
        self.queue_tries
    }

    /// The engine queue refused the call the last `tick` returned: it goes back to the front for
    /// the next tick, until it was refused `RELAY_CALL_QUEUE_TRIES` times in a row.
    pub fn queue_failed(&mut self, due: WaitingRelayCall<T>) -> QueueRetry<T> {
        self.queue_tries += 1;
        if self.queue_tries >= RELAY_CALL_QUEUE_TRIES {
            let tries = self.queue_tries;
            self.queue_tries = 0;
            self.notices.push(RelayCallNotice::QueueingGaveUp {
                name: due.call.function.clone(),
                tries,
            });
            return QueueRetry::GaveUp(due);
        }
        self.waiting.push_front(due);
        QueueRetry::Again {
            tries: self.queue_tries,
        }
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
        self.queue_tries = 0;
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

    /// Once per tick: the front call's wait in the waiting list, time with a menu open not
    /// counted. Past `RELAY_CALL_LONG_WAIT_LOGGED_EVERY_MS` it is returned, then once per that
    /// much more waiting; a new front starts at zero.
    pub fn long_wait(&mut self, menu_open: bool, now_ms: u64) -> Option<LongRelayWait> {
        let Some(front) = self.waiting.front() else {
            self.front_wait = None;
            return None;
        };
        let same_front = self.front_wait.as_ref().is_some_and(|wait| {
            wait.name == front.call.function && wait.submitted_ms == front.submitted_ms
        });
        if !same_front {
            self.front_wait = Some(FrontWait {
                name: front.call.function.clone(),
                submitted_ms: front.submitted_ms,
                counted_ms: 0,
                last_ms: now_ms,
                next_report_after_ms: RELAY_CALL_LONG_WAIT_LOGGED_EVERY_MS,
            });
            return None;
        }
        let waiting = self.waiting.len();
        let wait = self.front_wait.as_mut()?;
        if !menu_open {
            wait.counted_ms = wait
                .counted_ms
                .saturating_add(now_ms.saturating_sub(wait.last_ms));
        }
        wait.last_ms = now_ms;
        if wait.counted_ms <= wait.next_report_after_ms {
            return None;
        }
        wait.next_report_after_ms = wait.counted_ms + RELAY_CALL_LONG_WAIT_LOGGED_EVERY_MS;
        Some(LongRelayWait {
            front: wait.name.clone(),
            waited_ms: wait.counted_ms,
            waiting,
        })
    }

    pub fn take_notices(&mut self) -> Vec<RelayCallNotice> {
        std::mem::take(&mut self.notices)
    }
}
