use crate::callback_gate::{
    END_FOLLOW_STILL_WAITING_LOG_MS, EndDrop, EndFollow, PendingEnd, WaitStep, wait_step,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueEvent {
    Run {
        end: PendingEnd,
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
        self.entries.iter().any(|queued| queued.end.name == name)
    }

    pub fn push(&mut self, name: &str, phase: u16, now_ms: u64) -> u32 {
        if let Some(queued) = self.entries.iter().find(|queued| queued.end.name == name) {
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
        });
        self.arrivals
    }

    pub fn advance(
        &mut self,
        now_ms: u64,
        mut verdict_of: impl FnMut(&PendingEnd) -> EndFollow,
    ) -> Vec<QueueEvent> {
        let mut events = Vec::new();
        while let Some(front) = self.entries.first_mut() {
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

    pub fn take_through(&mut self, name: &str) -> Vec<(PendingEnd, u32)> {
        let Some(index) = self
            .entries
            .iter()
            .position(|queued| queued.end.name == name)
        else {
            return Vec::new();
        };
        let taken: Vec<(PendingEnd, u32)> = self
            .entries
            .drain(..=index)
            .map(|queued| (queued.end, queued.position))
            .collect();
        if self.entries.is_empty() {
            self.arrivals = 0;
        }
        taken
    }
}
