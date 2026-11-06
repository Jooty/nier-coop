use rc_proto::MAX_SCRIPT_ROLL_ORDINAL;

use crate::script_rolls::ROLL_ORDINAL_MAX_CALLS;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CallCounter {
    call_hash: u64,
    started_ms: u64,
    next: u16,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RollOrdinals {
    counters: Vec<CallCounter>,
}

impl RollOrdinals {
    pub const fn new() -> Self {
        Self {
            counters: Vec::new(),
        }
    }

    pub fn peek(&self, call_hash: u64, started_ms: u64) -> u16 {
        self.counters
            .iter()
            .find(|counter| counter.call_hash == call_hash && counter.started_ms == started_ms)
            .map_or(0, |counter| counter.next)
    }

    pub fn commit(&mut self, call_hash: u64, started_ms: u64) {
        let position = self
            .counters
            .iter()
            .position(|counter| counter.call_hash == call_hash);
        let counter = match position {
            Some(position) => &mut self.counters[position],
            None => {
                if self.counters.len() >= ROLL_ORDINAL_MAX_CALLS {
                    self.counters.remove(0);
                }
                self.counters.push(CallCounter {
                    call_hash,
                    started_ms,
                    next: 0,
                });
                let last = self.counters.len() - 1;
                &mut self.counters[last]
            }
        };
        if counter.started_ms != started_ms {
            counter.started_ms = started_ms;
            counter.next = 0;
        }
        counter.next = (counter.next + 1).min(MAX_SCRIPT_ROLL_ORDINAL);
    }

    pub fn clear(&mut self) {
        self.counters.clear();
    }
}
