use replicant_proto::StoryCall;

use crate::callback_gate::{
    END_FOLLOW_STILL_WAITING_LOG_MS, EndDrop, EndFollow, PendingEnd, WaitStep, wait_step,
};

const MAX_QUEUED_PARTNER_CALLS: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParkedKind {
    Run,
    Function,
    Handler,
    AcceptedTrigger,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParkedCall {
    pub kind: ParkedKind,
    pub call: StoryCall,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Released {
    End { end: PendingEnd, position: u32 },
    Call { parked: ParkedCall, position: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueEvent {
    Run {
        end: PendingEnd,
        position: u32,
        waited_ms: u64,
    },
    RunCall {
        parked: ParkedCall,
        position: u32,
        waited_ms: u64,
    },
    Dropped {
        end: PendingEnd,
        position: u32,
        reason: EndDrop,
    },
    StillWaiting {
        end: PendingEnd,
        position: u32,
        reason: EndDrop,
        waited_ms: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct QueuedEnd {
    end: PendingEnd,
    position: u32,
    minutes_logged: u64,
    parked: Option<ParkedCall>,
}

#[derive(Debug, Clone, Default)]
pub struct EndQueue {
    entries: Vec<QueuedEnd>,
    arrivals: u32,
}

impl EndQueue {
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
            arrivals: 0,
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.arrivals = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.entries
            .iter()
            .any(|queued| queued.parked.is_none() && queued.end.name == name)
    }

    pub fn push(&mut self, name: &str, phase: u16, now_ms: u64) -> u32 {
        if let Some(queued) = self
            .entries
            .iter()
            .find(|queued| queued.parked.is_none() && queued.end.name == name)
        {
            return queued.position;
        }
        self.arrivals += 1;
        self.entries.push(QueuedEnd {
            end: PendingEnd {
                name: name.to_owned(),
                phase,
                since_ms: now_ms,
            },
            position: self.arrivals,
            minutes_logged: 0,
            parked: None,
        });
        self.arrivals
    }

    pub fn park_call(&mut self, parked: ParkedCall, now_ms: u64) -> Option<u32> {
        if self.entries.is_empty() {
            return None;
        }
        self.park_call_always(parked, now_ms)
    }

    pub fn park_call_always(&mut self, parked: ParkedCall, now_ms: u64) -> Option<u32> {
        if self.entries.len() >= MAX_QUEUED_PARTNER_CALLS {
            return None;
        }
        self.arrivals += 1;
        self.entries.push(QueuedEnd {
            end: PendingEnd {
                name: parked.call.function.clone(),
                phase: parked.call.phase,
                since_ms: now_ms,
            },
            position: self.arrivals,
            minutes_logged: 0,
            parked: Some(parked),
        });
        Some(self.arrivals)
    }

    pub fn queued_function_names(&self, phase: u16) -> Vec<String> {
        self.entries
            .iter()
            .filter_map(|queued| queued.parked.as_ref())
            .filter(|parked| {
                matches!(parked.kind, ParkedKind::Function | ParkedKind::Handler)
                    && parked.call.phase == phase
            })
            .map(|parked| parked.call.function.clone())
            .collect()
    }

    pub fn front_end(&self) -> Option<&PendingEnd> {
        self.entries
            .iter()
            .find(|queued| queued.parked.is_none())
            .map(|queued| &queued.end)
    }

    pub fn clear_returning_calls(&mut self) -> usize {
        let dropped = self
            .entries
            .iter()
            .filter(|queued| queued.parked.is_some())
            .count();
        self.clear();
        dropped
    }

    pub fn advance(
        &mut self,
        now_ms: u64,
        mut verdict_of: impl FnMut(&PendingEnd) -> EndFollow,
    ) -> Vec<QueueEvent> {
        let mut events = Vec::new();
        while let Some(front) = self.entries.first_mut() {
            if front.parked.is_some() {
                let queued = self.entries.remove(0);
                if let Some(parked) = queued.parked {
                    events.push(QueueEvent::RunCall {
                        parked,
                        position: queued.position,
                        waited_ms: now_ms.saturating_sub(queued.end.since_ms),
                    });
                }
                continue;
            }
            let verdict = verdict_of(&front.end);
            match wait_step(front.end.since_ms, now_ms, verdict) {
                WaitStep::Run { waited_ms } => {
                    let queued = self.entries.remove(0);
                    events.push(QueueEvent::Run {
                        end: queued.end,
                        position: queued.position,
                        waited_ms,
                    });
                }
                WaitStep::Drop(reason) => {
                    let queued = self.entries.remove(0);
                    events.push(QueueEvent::Dropped {
                        end: queued.end,
                        position: queued.position,
                        reason,
                    });
                }
                WaitStep::KeepWaiting => {
                    let waited_ms = now_ms.saturating_sub(front.end.since_ms);
                    let minutes = waited_ms / END_FOLLOW_STILL_WAITING_LOG_MS;
                    if minutes > front.minutes_logged {
                        front.minutes_logged = minutes;
                        let EndFollow::Drop(reason) = verdict else {
                            break;
                        };
                        events.push(QueueEvent::StillWaiting {
                            end: front.end.clone(),
                            position: front.position,
                            reason,
                            waited_ms,
                        });
                    }
                    break;
                }
            }
        }
        if self.entries.is_empty() {
            self.arrivals = 0;
        }
        events
    }

    pub fn take_through(&mut self, name: &str) -> Vec<Released> {
        let index = self
            .entries
            .iter()
            .position(|queued| queued.parked.is_none() && queued.end.name == name);
        self.drain_through(index)
    }

    pub fn take_through_function(&mut self, name: &str) -> Vec<Released> {
        let index = self.entries.iter().position(|queued| {
            queued.parked.as_ref().is_some_and(|parked| {
                parked.kind == ParkedKind::Function && parked.call.function == name
            })
        });
        self.drain_through(index)
    }

    pub fn take_through_handler(&mut self, name: &str) -> Vec<Released> {
        let index = self.entries.iter().rposition(|queued| {
            queued.parked.as_ref().is_some_and(|parked| {
                parked.kind == ParkedKind::Handler && parked.call.function == name
            })
        });
        self.drain_through(index)
    }

    fn drain_through(&mut self, index: Option<usize>) -> Vec<Released> {
        let Some(index) = index else {
            return Vec::new();
        };
        let taken: Vec<Released> = self
            .entries
            .drain(..=index)
            .map(|queued| match queued.parked {
                Some(parked) => Released::Call {
                    parked,
                    position: queued.position,
                },
                None => Released::End {
                    end: queued.end,
                    position: queued.position,
                },
            })
            .collect();
        if self.entries.is_empty() {
            self.arrivals = 0;
        }
        taken
    }
}
