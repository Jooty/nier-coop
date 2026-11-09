use std::collections::BTreeMap;

use crate::fight_sync::{BASE_PLAYER_LOCAL_HANDLERS, StreamEntry};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotState {
    Idle,
    Busy,
    SteppingHeld,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopSource {
    Fired,
    Queued,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopVerdict {
    RunReplayed { seq: u64 },
    RunPlayerLocal,
    Hold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrival {
    Pending,
    Stale { current_epoch: u32 },
    Duplicate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InFlight {
    Idle,
    Queued { seq: u64 },
    Running { seq: u64 },
}

/// The guest's side of the stream. The engine runs one handler at a time and reuses the first
/// free queue slot, so at most one host entry is ever queued: entry N is released only once the
/// slot is idle and entry N-1 has popped and ended.
#[derive(Debug, Clone)]
pub struct Replayer {
    epoch: u32,
    next_seq: u64,
    pending: BTreeMap<u64, StreamEntry>,
    in_flight: InFlight,
    in_flight_entry: Option<StreamEntry>,
    player_local_names: Vec<String>,
    held_own_pops: u64,
    dropped_stale: u64,
}

impl Replayer {
    pub fn new<I, S>(extra_player_local_names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut player_local_names: Vec<String> = BASE_PLAYER_LOCAL_HANDLERS
            .iter()
            .map(|name| (*name).to_owned())
            .collect();
        player_local_names.extend(extra_player_local_names.into_iter().map(Into::into));
        Self {
            epoch: 0,
            next_seq: 0,
            pending: BTreeMap::new(),
            in_flight: InFlight::Idle,
            in_flight_entry: None,
            player_local_names,
            held_own_pops: 0,
            dropped_stale: 0,
        }
    }

    pub fn receive(&mut self, entry: StreamEntry) -> Arrival {
        if entry.epoch < self.epoch {
            self.dropped_stale += 1;
            return Arrival::Stale {
                current_epoch: self.epoch,
            };
        }
        if entry.epoch > self.epoch {
            self.restart(entry.epoch);
        }
        if entry.seq < self.next_seq || self.pending.contains_key(&entry.seq) {
            return Arrival::Duplicate;
        }
        if let InFlight::Queued { seq } | InFlight::Running { seq } = self.in_flight
            && seq == entry.seq
        {
            return Arrival::Duplicate;
        }
        self.pending.insert(entry.seq, entry);
        Arrival::Pending
    }

    /// Starts `epoch` if it is newer and returns how many pending entries it dropped. An entry
    /// already handed to the engine is forgotten; if it still pops, it pops as an unknown call.
    pub fn restart(&mut self, epoch: u32) -> usize {
        if epoch <= self.epoch {
            return 0;
        }
        let dropped = self.pending.len();
        self.dropped_stale += dropped as u64;
        self.pending.clear();
        self.epoch = epoch;
        self.next_seq = 0;
        self.in_flight = InFlight::Idle;
        self.in_flight_entry = None;
        dropped
    }

    pub fn pop(&mut self, name: &str, source: PopSource) -> PopVerdict {
        if source == PopSource::Queued
            && let InFlight::Queued { seq } = self.in_flight
            && self
                .in_flight_entry
                .as_ref()
                .is_some_and(|entry| entry.name == name)
        {
            self.in_flight = InFlight::Running { seq };
            return PopVerdict::RunReplayed { seq };
        }
        if self.is_player_local(name) {
            return PopVerdict::RunPlayerLocal;
        }
        self.held_own_pops += 1;
        PopVerdict::Hold
    }

    /// Called once per guest tick with the script slot's state; returns the entry to queue now.
    pub fn tick(&mut self, slot: SlotState) -> Option<StreamEntry> {
        if slot != SlotState::Idle {
            return None;
        }
        if let InFlight::Running { seq } = self.in_flight {
            self.next_seq = seq + 1;
            self.in_flight = InFlight::Idle;
            self.in_flight_entry = None;
        }
        if self.in_flight != InFlight::Idle {
            return None;
        }
        let mut entry = self.pending.remove(&self.next_seq)?;
        while self.is_player_local(&entry.name) {
            // The guest's engine fires its own copy of a player-local call.
            self.next_seq += 1;
            entry = self.pending.remove(&self.next_seq)?;
        }
        self.in_flight = InFlight::Queued { seq: entry.seq };
        self.in_flight_entry = Some(entry.clone());
        Some(entry)
    }

    /// The queued entry left the engine's queue without popping (the queue was cleared, or the
    /// call could not be queued): it goes back to pending and is queued again on the next tick.
    pub fn queued_call_lost(&mut self) -> Option<u64> {
        let InFlight::Queued { seq } = self.in_flight else {
            return None;
        };
        self.in_flight = InFlight::Idle;
        if let Some(entry) = self.in_flight_entry.take() {
            self.pending.insert(seq, entry);
        }
        Some(seq)
    }

    pub fn is_player_local(&self, name: &str) -> bool {
        self.player_local_names.iter().any(|local| local == name)
    }

    pub const fn in_flight(&self) -> InFlight {
        self.in_flight
    }

    pub const fn epoch(&self) -> u32 {
        self.epoch
    }

    pub const fn next_seq(&self) -> u64 {
        self.next_seq
    }

    pub fn in_flight_name(&self) -> Option<&str> {
        match self.in_flight {
            InFlight::Queued { .. } => self
                .in_flight_entry
                .as_ref()
                .map(|entry| entry.name.as_str()),
            InFlight::Idle | InFlight::Running { .. } => None,
        }
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    pub const fn held_own_pops(&self) -> u64 {
        self.held_own_pops
    }

    pub const fn dropped_stale(&self) -> u64 {
        self.dropped_stale
    }
}
