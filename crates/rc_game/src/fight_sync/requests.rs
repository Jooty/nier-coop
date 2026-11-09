use std::collections::VecDeque;

use rc_proto::StoryCallArg;

use crate::fight_sync::{PopSource, SAME_POP_ON_BOTH_GAMES_SPAN_MS, SlotState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestCall {
    pub name: String,
    pub args: Vec<StoryCallArg>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestReceipt {
    Waiting,
    /// The host's game popped the same call itself a moment ago; it is already in the stream.
    AlreadyRanHere,
    AlreadyWaiting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostPop {
    Run,
    /// The host's own game popped a call that ran for the guest's request more than the same-pop
    /// span ago in this epoch: it runs again here (logged once per name and epoch).
    RunAgainAfterRequest {
        since_request_ms: u64,
    },
    RunRequested,
    /// The host's own copy of a call it already ran for the guest's request.
    HoldRanByRequest,
    /// A queued request whose call the host's game popped itself before the request popped.
    HoldSuperseded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RequestInFlight {
    Idle,
    Queued { call: RequestCall, superseded: bool },
    Running,
}

/// The host's side of the guest's requests inside one epoch: one queued at a time, run once,
/// and never run when the host's own game popped the same call close to it.
#[derive(Debug, Clone)]
pub struct HostRequests {
    pending: VecDeque<RequestCall>,
    in_flight: RequestInFlight,
    own_pops: Vec<(String, u64)>,
    ran_for_requests: Vec<(String, u64)>,
    ran_for_requests_this_epoch: Vec<(String, u64)>,
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
            own_pops: Vec::new(),
            ran_for_requests: Vec::new(),
            ran_for_requests_this_epoch: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        *self = Self::new();
    }

    pub fn receive(&mut self, call: RequestCall, now_ms: u64) -> RequestReceipt {
        self.forget_old(now_ms);
        if self.own_pops.iter().any(|(name, _)| *name == call.name) {
            return RequestReceipt::AlreadyRanHere;
        }
        let queued_same = matches!(
            &self.in_flight,
            RequestInFlight::Queued { call: queued, .. } if queued.name == call.name
        );
        if queued_same || self.pending.iter().any(|waiting| waiting.name == call.name) {
            return RequestReceipt::AlreadyWaiting;
        }
        self.pending.push_back(call);
        RequestReceipt::Waiting
    }

    pub fn pop(&mut self, name: &str, source: PopSource, now_ms: u64) -> HostPop {
        self.forget_old(now_ms);
        if source == PopSource::Queued
            && let RequestInFlight::Queued { call, superseded } = &self.in_flight
            && call.name == name
        {
            if *superseded {
                // Nothing runs, so the slot stays idle and the next request may queue.
                self.in_flight = RequestInFlight::Idle;
                return HostPop::HoldSuperseded;
            }
            self.in_flight = RequestInFlight::Running;
            self.ran_for_requests.push((name.to_owned(), now_ms));
            self.ran_for_requests_this_epoch
                .push((name.to_owned(), now_ms));
            return HostPop::RunRequested;
        }
        if self.ran_for_requests.iter().any(|(ran, _)| ran == name) {
            return HostPop::HoldRanByRequest;
        }
        self.own_pops.push((name.to_owned(), now_ms));
        self.pending.retain(|waiting| waiting.name != name);
        if let RequestInFlight::Queued { call, superseded } = &mut self.in_flight
            && call.name == name
        {
            *superseded = true;
        }
        if let Some(index) = self
            .ran_for_requests_this_epoch
            .iter()
            .position(|(ran, _)| ran == name)
        {
            let (_, ran_at) = self.ran_for_requests_this_epoch.remove(index);
            return HostPop::RunAgainAfterRequest {
                since_request_ms: now_ms.saturating_sub(ran_at),
            };
        }
        HostPop::Run
    }

    /// Called once per host tick; returns the request to queue now. `other_calls_waiting`: another
    /// relay path has a call of ours in the queue, so this one waits for it.
    pub fn tick(
        &mut self,
        slot: SlotState,
        other_calls_waiting: bool,
        now_ms: u64,
    ) -> Option<RequestCall> {
        self.forget_old(now_ms);
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
        };
        Some(call)
    }

    /// The queued request left the engine's queue without popping: it is queued again next.
    pub fn queued_call_lost(&mut self) -> Option<String> {
        let RequestInFlight::Queued { call, .. } =
            std::mem::replace(&mut self.in_flight, RequestInFlight::Idle)
        else {
            return None;
        };
        let name = call.name.clone();
        self.pending.push_front(call);
        Some(name)
    }

    pub fn is_queued(&self) -> bool {
        matches!(self.in_flight, RequestInFlight::Queued { .. })
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    fn forget_old(&mut self, now_ms: u64) {
        let recent =
            |(_, at): &(String, u64)| now_ms.saturating_sub(*at) <= SAME_POP_ON_BOTH_GAMES_SPAN_MS;
        self.own_pops.retain(recent);
        self.ran_for_requests.retain(recent);
    }
}
