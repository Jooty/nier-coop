use rc_proto::{FightWindowChange, Message};

use crate::fight_sync::{
    Arrival, FIGHT_WINDOWS, FightWindowData, InFlight, PARTNER_CALL_POP_EXPECTED_MS, PopSource,
    PopVerdict, Replayer, SlotState, StreamEntry,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestPop {
    /// No host stream covers this game's phase: the pop takes the usual relay path.
    Outside,
    RunReplayed {
        seq: u64,
    },
    RunPlayerLocal,
    /// A call another relay path queued (an earlier partner call): it runs as that path decided.
    RunPartnerCall,
    Hold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestOpen {
    Joined(&'static FightWindowData),
    /// The guest is not in the window's phase, so this window's entries are not replayed here.
    NotHere,
    UnknownWindow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestArrival {
    Stream(Arrival),
    NotJoined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestReceived {
    Opened {
        open: GuestOpen,
        dropped_from_before: usize,
    },
    Closed {
        end_seq: u64,
    },
    Entry {
        seq: u64,
        arrival: GuestArrival,
    },
    NotAFightMessage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Joined {
    No,
    Yes {
        phase: u16,
        epoch: u32,
        window: &'static FightWindowData,
        end_seq: Option<u64>,
    },
}

/// The guest's side of one host fight window: it holds its own pops of the window's phase and
/// runs the host's entries one at a time in seq order, after any earlier partner calls.
#[derive(Debug, Clone)]
pub struct GuestFightStream {
    joined: Joined,
    replayer: Replayer,
    partner_calls_queued: Vec<(String, u64)>,
}

impl Default for GuestFightStream {
    fn default() -> Self {
        Self::new()
    }
}

impl GuestFightStream {
    pub fn new() -> Self {
        Self {
            joined: Joined::No,
            replayer: Replayer::new(Vec::<String>::new()),
            partner_calls_queued: Vec::new(),
        }
    }

    pub fn receive_message(&mut self, message: Message, local_phase: Option<u16>) -> GuestReceived {
        match message {
            Message::FightWindow(mark) => match mark.change {
                FightWindowChange::Opened => {
                    let dropped_from_before = self.replayer.pending_len();
                    let open = self.window_opened(
                        mark.phase,
                        mark.epoch,
                        mark.number,
                        mark.part,
                        local_phase,
                    );
                    GuestReceived::Opened {
                        open,
                        dropped_from_before,
                    }
                }
                FightWindowChange::Closed { end_seq } => {
                    self.window_closed(mark.epoch, end_seq);
                    GuestReceived::Closed { end_seq }
                }
            },
            Message::FightStream(entry) => {
                let seq = entry.seq;
                let arrival = self.receive(StreamEntry::from_wire(entry));
                GuestReceived::Entry { seq, arrival }
            }
            _ => GuestReceived::NotAFightMessage,
        }
    }

    pub fn window_opened(
        &mut self,
        phase: u16,
        epoch: u32,
        number: u16,
        part: u16,
        local_phase: Option<u16>,
    ) -> GuestOpen {
        self.leave();
        if local_phase != Some(phase) {
            return GuestOpen::NotHere;
        }
        let Some(window) = FIGHT_WINDOWS
            .iter()
            .find(|row| row.number == number && row.part == part)
        else {
            return GuestOpen::UnknownWindow;
        };
        self.replayer = Replayer::new(window.player_local.iter().copied());
        self.replayer.restart(epoch);
        self.joined = Joined::Yes {
            phase,
            epoch,
            window,
            end_seq: None,
        };
        GuestOpen::Joined(window)
    }

    pub fn window_closed(&mut self, epoch: u32, end_seq: u64) {
        if let Joined::Yes {
            epoch: joined_epoch,
            end_seq: closes_at,
            ..
        } = &mut self.joined
            && *joined_epoch == epoch
        {
            *closes_at = Some(end_seq);
        }
        self.leave_when_drained();
    }

    pub fn receive(&mut self, entry: StreamEntry) -> GuestArrival {
        let Joined::Yes { phase, epoch, .. } = self.joined else {
            return GuestArrival::NotJoined;
        };
        if entry.phase != phase || entry.epoch > epoch {
            return GuestArrival::NotJoined;
        }
        GuestArrival::Stream(self.replayer.receive(entry))
    }

    pub fn leave(&mut self) -> usize {
        let dropped = self.replayer.pending_len();
        self.joined = Joined::No;
        self.replayer = Replayer::new(Vec::<String>::new());
        dropped
    }

    pub fn is_active_in(&self, local_phase: Option<u16>) -> bool {
        matches!(self.joined, Joined::Yes { phase, .. } if Some(phase) == local_phase)
    }

    pub const fn window(&self) -> Option<&'static FightWindowData> {
        match self.joined {
            Joined::Yes { window, .. } => Some(window),
            Joined::No => None,
        }
    }

    pub const fn in_flight(&self) -> InFlight {
        self.replayer.in_flight()
    }

    pub fn pending_len(&self) -> usize {
        self.replayer.pending_len()
    }

    /// Another relay path queued `name` on this game; its pop runs, and host entries wait for it.
    pub fn partner_call_queued(&mut self, name: &str, now_ms: u64) {
        self.partner_calls_queued.push((name.to_owned(), now_ms));
    }

    pub fn partner_calls_queued(&self) -> usize {
        self.partner_calls_queued.len()
    }

    /// `registered_player_local`: the hook knows the name as a handler of a player-local message.
    pub fn pop(
        &mut self,
        name: &str,
        source: PopSource,
        local_phase: Option<u16>,
        registered_player_local: bool,
    ) -> GuestPop {
        if source == PopSource::Queued
            && let Some(index) = self
                .partner_calls_queued
                .iter()
                .position(|(queued, _)| queued == name)
            && !self.is_in_flight_name(name)
        {
            self.partner_calls_queued.remove(index);
            return GuestPop::RunPartnerCall;
        }
        if !self.is_active_in(local_phase) {
            return GuestPop::Outside;
        }
        let verdict = self.replayer.pop(name, source);
        match verdict {
            PopVerdict::RunReplayed { seq } => GuestPop::RunReplayed { seq },
            PopVerdict::RunPlayerLocal => GuestPop::RunPlayerLocal,
            PopVerdict::Hold if registered_player_local => GuestPop::RunPlayerLocal,
            PopVerdict::Hold => GuestPop::Hold,
        }
    }

    /// Called once per guest tick; returns the host entry to queue now, if any. Partner calls that
    /// were waiting before the window run first, so the entry waits while any are queued.
    pub fn tick(
        &mut self,
        slot: SlotState,
        local_phase: Option<u16>,
        earlier_partner_calls_waiting: bool,
        now_ms: u64,
    ) -> Option<StreamEntry> {
        self.partner_calls_queued
            .retain(|(_, at)| now_ms.saturating_sub(*at) <= PARTNER_CALL_POP_EXPECTED_MS);
        if !self.is_active_in(local_phase) {
            return None;
        }
        let partner_calls_first =
            earlier_partner_calls_waiting || !self.partner_calls_queued.is_empty();
        let releasable = !partner_calls_first || self.in_flight() != InFlight::Idle;
        let entry = if releasable {
            self.replayer.tick(slot)
        } else {
            None
        };
        self.leave_when_drained();
        entry
    }

    pub fn queued_call_lost(&mut self) -> Option<u64> {
        self.replayer.queued_call_lost()
    }

    fn is_in_flight_name(&self, name: &str) -> bool {
        self.replayer.in_flight_name() == Some(name)
    }

    fn leave_when_drained(&mut self) {
        let Joined::Yes {
            end_seq: Some(end_seq),
            ..
        } = self.joined
        else {
            return;
        };
        if self.replayer.next_seq() >= end_seq && self.replayer.in_flight() == InFlight::Idle {
            self.leave();
        }
    }
}
