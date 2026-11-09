use std::collections::VecDeque;

use rc_proto::StoryCallArg;

use crate::fight_sync::{MAX_WAITING_GUEST_REQUESTS, OwnQueuedCall, PopSource, SlotState};
use crate::story_relay::RELAY_CALL_QUEUE_TRIES;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestCall {
    pub name: String,
    pub args: Vec<StoryCallArg>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestReceipt {
    Waiting,
    /// The host's stream carried the call again after the guest's count of it: that entry
    /// already answered the request, so it is dropped.
    AlreadyRanHere,
    AlreadyWaiting,
    /// `MAX_WAITING_GUEST_REQUESTS` wait already; this one is not kept.
    TooManyWaiting,
    /// The name is no function of the open window: the guest's game cannot have popped it there.
    NotAWindowFunction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueRefusal {
    Again { tries: u32 },
    GaveUp { name: String, tries: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostPop {
    Run,
    RunRequested,
    /// The host's own copy of a call it already ran for the guest's request: each request run
    /// in this epoch holds one later own pop of that name.
    HoldRanByRequest,
    /// A queued request whose call the host's game popped itself before the request popped.
    HoldSuperseded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RequestInFlight {
    Idle,
    Queued {
        call: RequestCall,
        superseded: bool,
        arguments: Option<u64>,
        waited_ms: u64,
    },
    Running,
}

/// The host's side of the guest's requests inside one epoch: one queued at a time, run once,
/// and never run once the host's own game popped the same call. Copies pair by name and count.
#[derive(Debug, Clone)]
pub struct HostRequests {
    pending: VecDeque<RequestCall>,
    in_flight: RequestInFlight,
    queue_refusals: u32,
    ran_for_requests_unpaired: Vec<String>,
}

impl Default for HostRequests {
    fn default() -> Self {
        Self::new()
    }
}

impl HostRequests {
    pub const fn new() -> Self {
        Self {
            pending: VecDeque::new(),
            in_flight: RequestInFlight::Idle,
            queue_refusals: 0,
            ran_for_requests_unpaired: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        *self = Self::new();
    }

    pub fn receive(&mut self, call: RequestCall) -> RequestReceipt {
        let queued_same = matches!(
            &self.in_flight,
            RequestInFlight::Queued { call: queued, .. } if queued.name == call.name
        );
        if queued_same || self.pending.iter().any(|waiting| waiting.name == call.name) {
            return RequestReceipt::AlreadyWaiting;
        }
        if self.pending.len() >= MAX_WAITING_GUEST_REQUESTS {
            return RequestReceipt::TooManyWaiting;
        }
        self.pending.push_back(call);
        RequestReceipt::Waiting
    }

    /// `name` popped from the slot whose argument block is `arguments`; the queued request is
    /// matched by name and, once the hook said which slot it took, by that slot.
    pub fn pop(&mut self, name: &str, source: PopSource, arguments: u64) -> HostPop {
        if source == PopSource::Queued
            && let RequestInFlight::Queued {
                call,
                superseded,
                arguments: queued_at,
                ..
            } = &self.in_flight
            && call.name == name
            && queued_at.is_none_or(|queued| queued == arguments)
        {
            if *superseded {
                // Nothing runs, so the slot stays idle and the next request may queue.
                self.in_flight = RequestInFlight::Idle;
                return HostPop::HoldSuperseded;
            }
            self.in_flight = RequestInFlight::Running;
            self.ran_for_requests_unpaired.push(name.to_owned());
            return HostPop::RunRequested;
        }
        if let Some(index) = self
            .ran_for_requests_unpaired
            .iter()
            .position(|ran| ran == name)
        {
            self.ran_for_requests_unpaired.remove(index);
            return HostPop::HoldRanByRequest;
        }
        self.pending.retain(|waiting| waiting.name != name);
        if let RequestInFlight::Queued {
            call, superseded, ..
        } = &mut self.in_flight
            && call.name == name
        {
            *superseded = true;
        }
        HostPop::Run
    }

    /// Called once per host tick; returns the request to queue now. `other_calls_waiting`: another
    /// relay path has a call of ours in the queue, so this one waits for it.
    pub fn tick(&mut self, slot: SlotState, other_calls_waiting: bool) -> Option<RequestCall> {
        if slot != SlotState::Idle {
            return None;
        }
        if self.in_flight == RequestInFlight::Running {
            self.in_flight = RequestInFlight::Idle;
        }
        if self.in_flight != RequestInFlight::Idle || other_calls_waiting {
            return None;
        }
        let call = self.pending.pop_front()?;
        self.in_flight = RequestInFlight::Queued {
            call: call.clone(),
            superseded: false,
            arguments: None,
            waited_ms: 0,
        };
        Some(call)
    }

    /// The request the last tick returned is in the engine queue in the slot of `arguments`.
    pub fn queued_at(&mut self, at: u64) {
        self.queue_refusals = 0;
        if let RequestInFlight::Queued { arguments, .. } = &mut self.in_flight {
            *arguments = Some(at);
        }
    }

    /// The queued request as the hook placed it, once its slot is known, with its wait so far.
    pub fn queued_call(&self) -> Option<OwnQueuedCall> {
        let RequestInFlight::Queued {
            call,
            arguments: Some(arguments),
            waited_ms,
            ..
        } = &self.in_flight
        else {
            return None;
        };
        Some(OwnQueuedCall {
            name: call.name.clone(),
            arguments: *arguments,
            waited_ms: *waited_ms,
        })
    }

    /// Adds `counted_ms` to the queued request's wait and returns the wait so far.
    pub fn count_queued_wait(&mut self, counted_ms: u64) -> u64 {
        let RequestInFlight::Queued { waited_ms, .. } = &mut self.in_flight else {
            return 0;
        };
        *waited_ms = waited_ms.saturating_add(counted_ms);
        *waited_ms
    }

    /// The queued request stayed in the engine queue past the limit: it is dropped.
    pub fn give_up_queued(&mut self) -> Option<String> {
        if !self.is_queued() {
            return None;
        }
        let RequestInFlight::Queued { call, .. } =
            std::mem::replace(&mut self.in_flight, RequestInFlight::Idle)
        else {
            return None;
        };
        Some(call.name)
    }

    /// The queued request left the engine's queue without popping: it is queued again next.
    pub fn queued_call_lost(&mut self) -> Option<String> {
        if !self.is_queued() {
            return None;
        }
        let RequestInFlight::Queued { call, .. } =
            std::mem::replace(&mut self.in_flight, RequestInFlight::Idle)
        else {
            return None;
        };
        let name = call.name.clone();
        self.pending.push_front(call);
        Some(name)
    }

    /// The engine queue refused the request the last tick returned: it goes back to the front for
    /// the next tick, until it was refused `RELAY_CALL_QUEUE_TRIES` times in a row.
    pub fn queue_refused(&mut self) -> Option<QueueRefusal> {
        self.queue_refusals += 1;
        if self.queue_refusals >= RELAY_CALL_QUEUE_TRIES {
            let tries = self.queue_refusals;
            self.queue_refusals = 0;
            let name = self.give_up_queued()?;
            return Some(QueueRefusal::GaveUp { name, tries });
        }
        self.queued_call_lost()?;
        Some(QueueRefusal::Again {
            tries: self.queue_refusals,
        })
    }

    pub fn is_queued(&self) -> bool {
        matches!(self.in_flight, RequestInFlight::Queued { .. })
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }
}
