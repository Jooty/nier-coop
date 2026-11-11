use std::collections::VecDeque;

use replicant_proto::{FightRequestOutcome, FightRequestRefusal, StoryCallArg};

use crate::fight_sync::{MAX_WAITING_GUEST_REQUESTS, OwnQueuedCall, PopSource, SlotState};
use crate::story_relay::RELAY_CALL_QUEUE_TRIES;

/// The guest's own pop `copy_number` of `name` in this epoch, which it asked the host to run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestCall {
    pub name: String,
    pub args: Vec<StoryCallArg>,
    pub copy_number: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestReceipt {
    Waiting,
    /// The host's stream already carried the copy the guest asked for: that entry answered the
    /// request, so it is dropped.
    AlreadyRanHere,
    /// `MAX_WAITING_GUEST_REQUESTS` wait already; this one is not kept.
    TooManyWaiting,
    /// The name is no function of the open window: the guest's game cannot have popped it there.
    NotAWindowFunction,
}

/// What the host tells the guest about one request, once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestAnswer {
    pub name: String,
    pub copy_number: u32,
    pub outcome: FightRequestOutcome,
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
    /// The queued request popped from its slot after the host's stream already carried the copy
    /// it asked for (the host's game popped that call itself first): not run again.
    HoldSuperseded,
}

/// Told once each time the guest's requests start or stop waiting for the host's own scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneHoldChange {
    Started { waiting: usize },
    Over,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RequestInFlight {
    Idle,
    Queued {
        call: RequestCall,
        arguments: Option<u64>,
        waited_ms: u64,
    },
    Running,
}

/// The host's side of the guest's requests inside one epoch: one queued at a time, each run at
/// most once and answered once. A request's run is known by the queue slot it took.
#[derive(Debug, Clone)]
pub struct HostRequests {
    pending: VecDeque<RequestCall>,
    in_flight: RequestInFlight,
    queue_refusals: u32,
    answers: Vec<RequestAnswer>,
    held_by_own_scene: bool,
    scene_hold_change: Option<SceneHoldChange>,
}

impl RequestReceipt {
    pub const fn refusal(self) -> Option<FightRequestRefusal> {
        match self {
            Self::Waiting => None,
            Self::AlreadyRanHere => Some(FightRequestRefusal::AlreadyStreamed),
            Self::TooManyWaiting => Some(FightRequestRefusal::Full),
            Self::NotAWindowFunction => Some(FightRequestRefusal::NotAWindowFunction),
        }
    }
}

impl RequestAnswer {
    pub fn refused(call: &RequestCall, reason: FightRequestRefusal) -> Self {
        Self {
            name: call.name.clone(),
            copy_number: call.copy_number,
            outcome: FightRequestOutcome::Refused { reason },
        }
    }
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
            answers: Vec::new(),
            held_by_own_scene: false,
            scene_hold_change: None,
        }
    }

    /// The epoch or window ends: every request still waiting or queued is answered as one the
    /// fight moved on from. The answers stay until taken.
    pub fn forget(&mut self) {
        let mut answers = std::mem::take(&mut self.answers);
        if let RequestInFlight::Queued { call, .. } = &self.in_flight {
            answers.push(RequestAnswer::refused(
                call,
                FightRequestRefusal::FightMovedOn,
            ));
        }
        answers.extend(
            self.pending
                .iter()
                .map(|call| RequestAnswer::refused(call, FightRequestRefusal::FightMovedOn)),
        );
        *self = Self::new();
        self.answers = answers;
    }

    pub fn take_answers(&mut self) -> Vec<RequestAnswer> {
        std::mem::take(&mut self.answers)
    }

    /// Each request is kept on its own, even several of one name: each is one touch.
    pub fn receive(&mut self, call: RequestCall) -> RequestReceipt {
        if self.pending.len() >= MAX_WAITING_GUEST_REQUESTS {
            return RequestReceipt::TooManyWaiting;
        }
        self.pending.push_back(call);
        RequestReceipt::Waiting
    }

    /// `name` popped from the slot whose argument block is `arguments`; `streamed_copies` of it
    /// were streamed so far in this epoch. Only the slot the queued request took is its run; any
    /// other pop of the name is the host's own and runs.
    pub fn pop(
        &mut self,
        name: &str,
        source: PopSource,
        arguments: u64,
        streamed_copies: u32,
    ) -> HostPop {
        let is_request_slot = source == PopSource::Queued
            && matches!(
                &self.in_flight,
                RequestInFlight::Queued {
                    call,
                    arguments: Some(queued_at),
                    ..
                } if call.name == name && *queued_at == arguments
            );
        if !is_request_slot {
            return HostPop::Run;
        }
        let RequestInFlight::Queued { call, .. } =
            std::mem::replace(&mut self.in_flight, RequestInFlight::Running)
        else {
            return HostPop::Run;
        };
        if streamed_copies >= call.copy_number {
            // Nothing runs, so the slot stays idle and the next request may queue.
            self.in_flight = RequestInFlight::Idle;
            self.answers.push(RequestAnswer::refused(
                &call,
                FightRequestRefusal::AlreadyStreamed,
            ));
            return HostPop::HoldSuperseded;
        }
        self.answers.push(RequestAnswer {
            name: call.name,
            copy_number: call.copy_number,
            outcome: FightRequestOutcome::Ran,
        });
        HostPop::RunRequested
    }

    /// Called once per host tick; returns the request to queue now, none while another relay path
    /// has a call queued (`other_calls_waiting`) or the host's own event scene plays. A waiting
    /// request whose copy `streamed_copies` already reached is answered and dropped instead.
    pub fn tick(
        &mut self,
        slot: SlotState,
        other_calls_waiting: bool,
        own_scene_playing: bool,
        streamed_copies: impl Fn(&str) -> u32,
    ) -> Option<RequestCall> {
        self.note_scene_hold(own_scene_playing);
        if slot != SlotState::Idle {
            return None;
        }
        if self.in_flight == RequestInFlight::Running {
            self.in_flight = RequestInFlight::Idle;
        }
        if self.in_flight != RequestInFlight::Idle || other_calls_waiting || own_scene_playing {
            return None;
        }
        let call = loop {
            let call = self.pending.pop_front()?;
            if streamed_copies(&call.name) < call.copy_number {
                break call;
            }
            self.answers.push(RequestAnswer::refused(
                &call,
                FightRequestRefusal::AlreadyStreamed,
            ));
        };
        self.in_flight = RequestInFlight::Queued {
            call: call.clone(),
            arguments: None,
            waited_ms: 0,
        };
        Some(call)
    }

    pub fn take_scene_hold_change(&mut self) -> Option<SceneHoldChange> {
        self.scene_hold_change.take()
    }

    fn note_scene_hold(&mut self, own_scene_playing: bool) {
        let held = own_scene_playing && !self.pending.is_empty();
        if held == self.held_by_own_scene {
            return;
        }
        self.held_by_own_scene = held;
        self.scene_hold_change = Some(if held {
            SceneHoldChange::Started {
                waiting: self.pending.len(),
            }
        } else {
            SceneHoldChange::Over
        });
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

    /// The queued request stayed in the engine queue past the limit, or the queue kept refusing
    /// it: it is dropped and answered as given up.
    pub fn give_up_queued(&mut self) -> Option<String> {
        if !self.is_queued() {
            return None;
        }
        let RequestInFlight::Queued { call, .. } =
            std::mem::replace(&mut self.in_flight, RequestInFlight::Idle)
        else {
            return None;
        };
        self.answers
            .push(RequestAnswer::refused(&call, FightRequestRefusal::GaveUp));
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
