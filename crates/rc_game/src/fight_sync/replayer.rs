use std::collections::BTreeMap;

use crate::fight_sync::{BASE_PLAYER_LOCAL_HANDLERS, OwnQueuedCall, StreamEntry};

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
/// slot is idle and entry N-1 has popped and ended. A queued entry is ours only in the slot it
/// took (`entry_queued_at`) under its name; one forgotten by a restart is handed out by
/// `take_forgotten`, since it may still be in the engine queue.
#[derive(Debug, Clone)]
pub struct Replayer {
    epoch: u32,
    next_seq: u64,
    pending: BTreeMap<u64, StreamEntry>,
    in_flight: InFlight,
    in_flight_entry: Option<StreamEntry>,
    queued_arguments: Option<u64>,
    forgotten: Vec<OwnQueuedCall>,
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
            queued_arguments: None,
            forgotten: Vec::new(),
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
    /// already handed to the engine is forgotten here and handed out by `take_forgotten`.
    pub fn restart(&mut self, epoch: u32) -> usize {
        if epoch <= self.epoch {
            return 0;
        }
        let dropped = self.pending.len();
        self.dropped_stale += dropped as u64;
        self.pending.clear();
        self.epoch = epoch;
        self.next_seq = 0;
        self.forget_in_flight();
        dropped
    }

    /// The guest's own phase restarted: entries waiting or handed to its engine are of no use
    /// now. The epoch stays; the host's next epoch replaces it.
    pub fn drop_waiting(&mut self) -> usize {
        let dropped = self.pending.len() + usize::from(self.in_flight != InFlight::Idle);
        self.dropped_stale += dropped as u64;
        self.pending.clear();
        self.forget_in_flight();
        dropped
    }

    /// Entry `seq` is not run here: true when it was the next one and nothing is in flight.
    pub fn skip_entry(&mut self, seq: u64) -> bool {
        if self.in_flight != InFlight::Idle
            || seq != self.next_seq
            || self.pending.contains_key(&seq)
        {
            return false;
        }
        self.next_seq += 1;
        true
    }

    /// The engine popped `name` from the slot whose argument block is `arguments`.
    pub fn pop(&mut self, name: &str, source: PopSource, arguments: u64) -> PopVerdict {
        if source == PopSource::Queued
            && let InFlight::Queued { seq } = self.in_flight
            && self.is_queued_at(name, arguments)
        {
            self.in_flight = InFlight::Running { seq };
            self.queued_arguments = None;
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
        self.tick_when(slot, |_| true)
    }

    /// As `tick`, but the entry that is next stays pending while `may_release(entry)` is false.
    pub fn tick_when(
        &mut self,
        slot: SlotState,
        mut may_release: impl FnMut(&StreamEntry) -> bool,
    ) -> Option<StreamEntry> {
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
        if !may_release(&entry) {
            self.pending.insert(entry.seq, entry);
            return None;
        }
        self.in_flight = InFlight::Queued { seq: entry.seq };
        self.in_flight_entry = Some(entry.clone());
        Some(entry)
    }

    /// The entry the last tick released is in the engine queue in the slot of `arguments`.
    pub fn entry_queued_at(&mut self, arguments: u64) {
        if matches!(self.in_flight, InFlight::Queued { .. }) {
            self.queued_arguments = Some(arguments);
        }
    }

    /// The queued entry as the hook placed it, once its slot is known.
    pub fn queued_call(&self) -> Option<OwnQueuedCall> {
        let InFlight::Queued { .. } = self.in_flight else {
            return None;
        };
        let name = &self.in_flight_entry.as_ref()?.name;
        Some(OwnQueuedCall::new(name, self.queued_arguments?))
    }

    /// True when `name` popping from the slot of `arguments` is the queued entry: same name, and
    /// the same slot once the hook said which one it took.
    pub fn is_queued_at(&self, name: &str, arguments: u64) -> bool {
        matches!(self.in_flight, InFlight::Queued { .. })
            && self
                .in_flight_entry
                .as_ref()
                .is_some_and(|entry| entry.name == name)
            && self
                .queued_arguments
                .is_none_or(|queued| queued == arguments)
    }

    /// The queued entry stayed in the engine queue past the limit: it is skipped, so the replay
    /// goes on with the next one.
    pub fn give_up_queued(&mut self) -> Option<u64> {
        let InFlight::Queued { seq } = self.in_flight else {
            return None;
        };
        self.next_seq = seq + 1;
        self.in_flight = InFlight::Idle;
        self.in_flight_entry = None;
        self.queued_arguments = None;
        Some(seq)
    }

    /// Entries the engine may still hold that a restart or a drop forgot here.
    pub fn take_forgotten(&mut self) -> Vec<OwnQueuedCall> {
        std::mem::take(&mut self.forgotten)
    }

    /// Hands the queued entry to the caller and forgets it here, as when the window is left.
    pub fn forget_queued(&mut self) -> Option<OwnQueuedCall> {
        let call = self.queued_call();
        self.in_flight = InFlight::Idle;
        self.in_flight_entry = None;
        self.queued_arguments = None;
        call
    }

    /// The queued entry left the engine's queue without popping (the queue was cleared, or the
    /// call could not be queued): it goes back to pending and is queued again on the next tick.
    pub fn queued_call_lost(&mut self) -> Option<u64> {
        let InFlight::Queued { seq } = self.in_flight else {
            return None;
        };
        self.in_flight = InFlight::Idle;
        self.queued_arguments = None;
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

    /// The lowest pending entry, by seq and name.
    pub fn first_pending(&self) -> Option<(u64, &str)> {
        self.pending
            .first_key_value()
            .map(|(seq, entry)| (*seq, entry.name.as_str()))
    }

    pub const fn held_own_pops(&self) -> u64 {
        self.held_own_pops
    }

    pub const fn dropped_stale(&self) -> u64 {
        self.dropped_stale
    }

    fn forget_in_flight(&mut self) {
        if let Some(call) = self.queued_call() {
            self.forgotten.push(call);
        }
        self.in_flight = InFlight::Idle;
        self.in_flight_entry = None;
        self.queued_arguments = None;
    }
}
