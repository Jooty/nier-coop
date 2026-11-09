use std::collections::{HashMap, HashSet};

use rc_proto::{
    FightRequest, FightRequestKind, FightTape, FightWindowChange, Message, StoryCallArg,
};

use crate::fight_sync::{
    Arrival, EarlierQueuedCalls, FIGHT_WINDOWS, FightWindowData, GuestTape,
    HOST_ENTRY_END_WAIT_LIMIT_MS, HOST_ENTRY_END_WAIT_LOGGED_FROM_MS, InFlight,
    MAX_HELD_NEXT_WINDOW_MESSAGES, NEXT_WINDOW_HELD_AT_MOST_MS, ONE_RESTART_SIGNALS_SPAN_MS,
    OWN_POP_HELD_FOR_HOST_COPY_MS, PARTNER_CALL_POP_EXPECTED_MS, PARTNER_RESTART_EXPECTED_MS,
    PopSource, PopVerdict, QUEUED_CALL_GIVEN_UP_AFTER_MS, QueuedCallNotice, Replayer, SlotState,
    StreamEntry, TapeKey, TapeValue, TapedResult, counted_wait_ms,
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
    /// A host entry this game queued for a window or epoch it has since left: the host ran it,
    /// so it runs here too.
    RunEarlierEntry,
    Hold,
    /// A pop only this game's player caused, with no host copy of it to pair with yet: not run
    /// here. A host copy that comes later pairs with it; else it goes to the host as a request
    /// once `OWN_POP_HELD_FOR_HOST_COPY_MS` passed, and the host runs it once and streams it back.
    HeldForHostCopy,
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
    /// Entry 0 is the pop that armed the host's window, and this game already ran its own.
    ArmingEntrySkipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestReceived {
    Opened {
        open: GuestOpen,
        dropped_from_before: usize,
    },
    Restarted(GuestRestart),
    Closed {
        end_seq: u64,
    },
    Entry {
        seq: u64,
        arrival: GuestArrival,
    },
    /// A host tape value; `kept` is false when it is for a fight or epoch this game is not in.
    Tape {
        seq: u64,
        kept: bool,
    },
    /// The host's handler of entry `seq` ended; `kept` as for a tape value.
    EntryEnded {
        seq: u64,
        kept: bool,
    },
    /// A message of the host's next window, kept until this game can follow it: the window it
    /// replays drained, or it reached the host's new phase. `first` for the one that began it.
    HeldForNextWindow {
        first: bool,
    },
    NotAFightMessage,
}

/// A taped native's value inside a replayed handler: the host's when it taped the same read,
/// else the local one; `log` is set once per native and entry.
#[derive(Debug, Clone, PartialEq)]
pub struct GuestTapeRead {
    pub value: TapeValue,
    pub log: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestRestart {
    /// Both games restarted: the replay goes on at the host's new epoch.
    Paired {
        epoch: u32,
    },
    /// This game restarted first; the host's new epoch is awaited.
    WaitingForHost,
    /// The host restarted first; this game's own restart is awaited.
    WaitingForOwnRestart {
        epoch: u32,
    },
    /// Another restart signal of a restart already counted.
    SameRestart,
    /// A host restart mark for a window this game had not joined: joined now at its epoch.
    Joined(GuestOpen),
    NotInFight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartWait {
    None,
    ForHost { since_ms: u64, request_sent: bool },
    ForOwnRestart { since_ms: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuestNotice {
    /// No host restart came in time after this game's own: the host was asked to restart.
    RestartRequested { epoch: u32 },
    /// The host's restart was not followed by one here in time; the replay goes on.
    HostRestartedAlone { epoch: u32 },
    /// The host never restarted after the request; this game left the fight's stream.
    LeftWithoutHostRestart { dropped: usize },
    /// Entry `seq` waited `waited_ms` for the host's handler of it to end before it was queued.
    WaitedForHostEntryEnd { seq: u64, waited_ms: u64 },
    /// The host's end of entry `seq` never came within the wait limit; it was queued anyway, so a
    /// tape value the host reads later is not used here.
    HostEntryEndNeverCame { seq: u64, waited_ms: u64 },
    /// This game's own pop `name` was held inside the window and the host's stream never carried
    /// a copy to pair with it in the epoch, nor was it sent to the host (logged once per name
    /// and epoch, when the epoch ends).
    HeldPopNeverMatched { name: String, epoch: u32 },
    /// This game's player pop `name` waited `held_ms` and the host's copy of it came: paired,
    /// no request.
    PlayerPopPairedWithHostCopy { name: String, held_ms: u64 },
    /// No host copy of this game's player pop `name` came in time: sent to the host as a
    /// request, with the host copies of it seen in this epoch.
    RequestSent { name: String, host_copies_seen: u32 },
    /// The host's entry 0 `name` armed its window; this game's own copy had already run, so the
    /// entry was skipped.
    ArmingEntrySkipped { name: String, epoch: u32 },
    /// The host's next window waited this long and this game still could not follow it: the
    /// earlier window is left undrained, or the host's phase was never reached.
    NextWindowWaitedTooLong { waited_ms: u64 },
    /// More messages of the host's next window came than are kept while it waits.
    NextWindowMessagesDropped { dropped: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeldUntil {
    EarlierWindowDrained,
    Phase(u16),
}

#[derive(Debug, Clone)]
struct HeldNextWindow {
    until: HeldUntil,
    since_ms: Option<u64>,
    messages: Vec<Message>,
    dropped: usize,
}

#[derive(Debug, Clone, PartialEq)]
struct PlayerPopHeld {
    name: String,
    args: Vec<StoryCallArg>,
    since_ms: u64,
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
    requests_out: Vec<FightRequest>,
    requests_unanswered: Vec<String>,
    host_copies: HashMap<String, u32>,
    own_copies: HashMap<String, u32>,
    player_pops_held: Vec<PlayerPopHeld>,
    restart_wait: RestartWait,
    last_own_restart_ms: Option<u64>,
    notices: Vec<GuestNotice>,
    last_tick_ms: u64,
    tape: GuestTape,
    tape_hits_logged: HashSet<(u64, String)>,
    host_entries_ended_through: Option<u64>,
    waiting_for_host_entry_end: Option<(u64, u64)>,
    held_pops_unmatched: Vec<String>,
    held_pops_logged: HashSet<String>,
    ran_outside_window_phase: Option<u16>,
    ran_outside_window: HashSet<String>,
    arming_entry_to_skip: Option<String>,
    earlier_queued: EarlierQueuedCalls,
    queued_waited_ms: u64,
    last_queue_check_ms: Option<u64>,
    next_window: Option<HeldNextWindow>,
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
            requests_out: Vec::new(),
            requests_unanswered: Vec::new(),
            host_copies: HashMap::new(),
            own_copies: HashMap::new(),
            player_pops_held: Vec::new(),
            restart_wait: RestartWait::None,
            last_own_restart_ms: None,
            notices: Vec::new(),
            last_tick_ms: 0,
            tape: GuestTape::new(),
            tape_hits_logged: HashSet::new(),
            host_entries_ended_through: None,
            waiting_for_host_entry_end: None,
            held_pops_unmatched: Vec::new(),
            held_pops_logged: HashSet::new(),
            ran_outside_window_phase: None,
            ran_outside_window: HashSet::new(),
            arming_entry_to_skip: None,
            earlier_queued: EarlierQueuedCalls::new(),
            queued_waited_ms: 0,
            last_queue_check_ms: None,
            next_window: None,
        }
    }

    /// A fight message from the host. One that opens the host's next window while this game
    /// still drains the closed one, or that moves the window to a phase this game has not
    /// reached yet, is held with every message after it until `release_next_window` lets it go.
    pub fn receive_message(&mut self, message: Message, local_phase: Option<u16>) -> GuestReceived {
        if !matches!(
            message,
            Message::FightWindow(_) | Message::FightStream(_) | Message::FightTape(_)
        ) {
            return GuestReceived::NotAFightMessage;
        }
        if let Some(held) = &mut self.next_window {
            if held.messages.len() < MAX_HELD_NEXT_WINDOW_MESSAGES {
                held.messages.push(message);
            } else {
                held.dropped += 1;
            }
            return GuestReceived::HeldForNextWindow { first: false };
        }
        if let Some(until) = self.next_window_wait(&message, local_phase) {
            self.next_window = Some(HeldNextWindow {
                until,
                since_ms: None,
                messages: vec![message],
                dropped: 0,
            });
            return GuestReceived::HeldForNextWindow { first: true };
        }
        self.handle_message(message, local_phase)
    }

    /// Once per guest tick, before the replay: lets the held next window go once this game can
    /// follow it (or the wait ran out) and returns what each held message did.
    pub fn release_next_window(
        &mut self,
        local_phase: Option<u16>,
        now_ms: u64,
    ) -> Vec<GuestReceived> {
        let Some(held) = &mut self.next_window else {
            return Vec::new();
        };
        let since_ms = *held.since_ms.get_or_insert(now_ms);
        let until = held.until;
        let ready = match until {
            HeldUntil::EarlierWindowDrained => !self.is_draining(),
            HeldUntil::Phase(phase) => local_phase == Some(phase),
        };
        let waited_ms = now_ms.saturating_sub(since_ms);
        if !ready && waited_ms <= NEXT_WINDOW_HELD_AT_MOST_MS {
            return Vec::new();
        }
        let Some(held) = self.next_window.take() else {
            return Vec::new();
        };
        if !ready {
            self.notices
                .push(GuestNotice::NextWindowWaitedTooLong { waited_ms });
            if until == HeldUntil::EarlierWindowDrained {
                self.leave();
            }
        }
        if held.dropped > 0 {
            self.notices.push(GuestNotice::NextWindowMessagesDropped {
                dropped: held.dropped,
            });
        }
        held.messages
            .into_iter()
            .map(|message| self.receive_message(message, local_phase))
            .collect()
    }

    fn handle_message(&mut self, message: Message, local_phase: Option<u16>) -> GuestReceived {
        match message {
            Message::FightWindow(mark) => match mark.change {
                FightWindowChange::Opened { armed_by } => {
                    let dropped_from_before = self.replayer.pending_len();
                    let open = self.window_opened(
                        mark.phase,
                        mark.epoch,
                        mark.number,
                        mark.part,
                        local_phase,
                        armed_by.as_deref(),
                    );
                    GuestReceived::Opened {
                        open,
                        dropped_from_before,
                    }
                }
                FightWindowChange::Restarted => GuestReceived::Restarted(self.host_restarted(
                    mark.phase,
                    mark.epoch,
                    mark.number,
                    mark.part,
                    local_phase,
                )),
                FightWindowChange::Closed { end_seq } => {
                    self.window_closed(mark.epoch, end_seq);
                    GuestReceived::Closed { end_seq }
                }
                FightWindowChange::EntryEnded { seq } => GuestReceived::EntryEnded {
                    seq,
                    kept: self.host_entry_ended(mark.phase, mark.epoch, seq),
                },
            },
            Message::FightStream(entry) => {
                let seq = entry.seq;
                let arrival = self.receive(StreamEntry::from_wire(entry));
                GuestReceived::Entry { seq, arrival }
            }
            Message::FightTape(tape) => {
                let seq = tape.seq;
                let kept = self.receive_tape(tape);
                GuestReceived::Tape { seq, kept }
            }
            _ => GuestReceived::NotAFightMessage,
        }
    }

    /// `armed_by`: the arm id whose pop opened the host's window (its entry 0). When this game
    /// already ran its own copy since its last window in this phase, that entry is skipped.
    pub fn window_opened(
        &mut self,
        phase: u16,
        epoch: u32,
        number: u16,
        part: u16,
        local_phase: Option<u16>,
        armed_by: Option<&str>,
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
        self.arming_entry_to_skip = armed_by
            .filter(|name| {
                self.ran_outside_window_phase == Some(phase)
                    && self.ran_outside_window.contains(*name)
            })
            .map(str::to_owned);
        GuestOpen::Joined(window)
    }

    /// The host's phase restarted inside its open window and its epoch moved on.
    pub fn host_restarted(
        &mut self,
        phase: u16,
        epoch: u32,
        number: u16,
        part: u16,
        local_phase: Option<u16>,
    ) -> GuestRestart {
        let Joined::Yes {
            phase: joined_phase,
            epoch: joined_epoch,
            window,
            ..
        } = self.joined
        else {
            return GuestRestart::Joined(self.window_opened(
                phase,
                epoch,
                number,
                part,
                local_phase,
                None,
            ));
        };
        if joined_phase != phase || (window.number, window.part) != (number, part) {
            return GuestRestart::Joined(self.window_opened(
                phase,
                epoch,
                number,
                part,
                local_phase,
                None,
            ));
        }
        if epoch <= joined_epoch {
            return GuestRestart::SameRestart;
        }
        self.replayer.restart(epoch);
        self.keep_forgotten_calls();
        self.clear_tape();
        let held = std::mem::take(&mut self.held_pops_unmatched);
        self.note_held_pops_never_matched(held, joined_epoch);
        self.forget_epoch_marks();
        self.joined = Joined::Yes {
            phase,
            epoch,
            window,
            end_seq: None,
        };
        self.forget_copies();
        self.arming_entry_to_skip = None;
        if let RestartWait::ForHost { .. } = self.restart_wait {
            self.restart_wait = RestartWait::None;
            return GuestRestart::Paired { epoch };
        }
        self.restart_wait = RestartWait::ForOwnRestart {
            since_ms: self.last_tick_ms,
        };
        GuestRestart::WaitingForOwnRestart { epoch }
    }

    /// This game's phase restarted (its engine dropped every queued call). Inside a joined window
    /// the stream is kept: the host's new epoch carries the fight on.
    pub fn own_restart(&mut self, now_ms: u64) -> GuestRestart {
        self.ran_outside_window.clear();
        let Joined::Yes { epoch, .. } = self.joined else {
            return GuestRestart::NotInFight;
        };
        let same_restart = self
            .last_own_restart_ms
            .is_some_and(|at| now_ms.saturating_sub(at) <= ONE_RESTART_SIGNALS_SPAN_MS);
        self.last_own_restart_ms = Some(now_ms);
        if same_restart {
            return GuestRestart::SameRestart;
        }
        if let RestartWait::ForOwnRestart { .. } = self.restart_wait {
            self.restart_wait = RestartWait::None;
            return GuestRestart::Paired { epoch };
        }
        self.replayer.drop_waiting();
        self.keep_forgotten_calls();
        self.clear_tape();
        self.requests_unanswered.clear();
        self.player_pops_held.clear();
        self.restart_wait = RestartWait::ForHost {
            since_ms: now_ms,
            request_sent: false,
        };
        GuestRestart::WaitingForHost
    }

    pub const fn restart_wait(&self) -> RestartWait {
        self.restart_wait
    }

    pub fn window_closed(&mut self, epoch: u32, end_seq: u64) {
        if let RestartWait::ForHost { .. } = self.restart_wait {
            // The host's fight ended while this game waited for its restart: nothing to resume.
            self.leave();
            return;
        }
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
        if self.skips_arming_entry(&entry, epoch) {
            return GuestArrival::ArmingEntrySkipped;
        }
        let name = entry.name.clone();
        let arrival = self.replayer.receive(entry);
        if arrival == Arrival::Pending {
            self.host_copy_arrived(&name);
        }
        GuestArrival::Stream(arrival)
    }

    /// Leaves the joined window. An entry still in the engine queue stays tracked as an earlier
    /// call, so nothing of the next window is queued beside it.
    pub fn leave(&mut self) -> usize {
        let dropped = self.replayer.pending_len();
        if let Some(mut call) = self.replayer.forget_queued() {
            call.waited_ms = self.queued_waited_ms;
            self.earlier_queued.add(call);
        }
        self.keep_forgotten_calls();
        if let Joined::Yes { epoch, .. } = self.joined {
            let held = std::mem::take(&mut self.held_pops_unmatched);
            self.note_held_pops_never_matched(held, epoch);
            self.ran_outside_window.clear();
        }
        self.forget_epoch_marks();
        self.joined = Joined::No;
        self.replayer = Replayer::new(Vec::<String>::new());
        self.requests_out.clear();
        self.forget_copies();
        self.arming_entry_to_skip = None;
        self.restart_wait = RestartWait::None;
        self.last_own_restart_ms = None;
        self.clear_tape();
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

    /// `name` popped from the slot whose argument block is `arguments`.
    /// `registered_player_local`: the hook knows the name as a handler of a player-local message.
    /// A pop that runs while no window is joined is noted, so the host's streamed copy of the
    /// pop that armed its next window is skipped here.
    pub fn pop(
        &mut self,
        name: &str,
        source: PopSource,
        arguments: u64,
        local_phase: Option<u16>,
        registered_player_local: bool,
    ) -> GuestPop {
        let verdict = self.judge_pop(
            name,
            source,
            arguments,
            local_phase,
            registered_player_local,
        );
        self.note_ran_outside_window(name, verdict, local_phase);
        verdict
    }

    fn judge_pop(
        &mut self,
        name: &str,
        source: PopSource,
        arguments: u64,
        local_phase: Option<u16>,
        registered_player_local: bool,
    ) -> GuestPop {
        // The slot runs one handler at a time, so any pop ends the replayed handler's tape.
        self.tape.end();
        if source == PopSource::Queued && self.earlier_queued.popped(name, arguments) {
            return GuestPop::RunEarlierEntry;
        }
        if source == PopSource::Queued
            && let Some(index) = self
                .partner_calls_queued
                .iter()
                .position(|(queued, _)| queued == name)
            && !self.replayer.is_queued_at(name, arguments)
        {
            self.partner_calls_queued.remove(index);
            return GuestPop::RunPartnerCall;
        }
        if !self.is_active_in(local_phase) {
            return GuestPop::Outside;
        }
        let verdict = self.replayer.pop(name, source, arguments);
        match verdict {
            PopVerdict::RunReplayed { seq } => {
                self.tape.begin(seq);
                GuestPop::RunReplayed { seq }
            }
            PopVerdict::RunPlayerLocal => GuestPop::RunPlayerLocal,
            PopVerdict::Hold if registered_player_local => GuestPop::RunPlayerLocal,
            PopVerdict::Hold => {
                if !self.own_copy_pairs(name) {
                    self.held_pops_unmatched.push(name.to_owned());
                }
                GuestPop::Hold
            }
        }
    }

    /// A pop fired by this game's own player (a trap, an action target, a player-move callback).
    /// Inside the window each host copy of a name pairs with one own pop of it, whenever either
    /// came: a pop the stream already carries, or one a request of it still waits for, is held;
    /// any other is held until a host copy pairs with it or the hold time makes it a request.
    pub fn pop_by_player(
        &mut self,
        name: &str,
        args: Vec<StoryCallArg>,
        local_phase: Option<u16>,
        registered_player_local: bool,
        now_ms: u64,
    ) -> GuestPop {
        self.tape.end();
        if !self.is_active_in(local_phase) {
            self.note_ran_outside_window(name, GuestPop::Outside, local_phase);
            return GuestPop::Outside;
        }
        if registered_player_local || self.replayer.is_player_local(name) {
            return GuestPop::RunPlayerLocal;
        }
        if self.restart_wait != RestartWait::None {
            return GuestPop::Hold;
        }
        // A repeat while an own pop of the name already waits (held or asked) is the same touch.
        if self.requests_unanswered.iter().any(|asked| asked == name)
            || self.player_pops_held.iter().any(|held| held.name == name)
        {
            return GuestPop::Hold;
        }
        if self.own_copy_pairs(name) {
            return GuestPop::Hold;
        }
        self.player_pops_held.push(PlayerPopHeld {
            name: name.to_owned(),
            args,
            since_ms: now_ms,
        });
        GuestPop::HeldForHostCopy
    }

    pub fn take_requests(&mut self) -> Vec<FightRequest> {
        std::mem::take(&mut self.requests_out)
    }

    pub fn take_notices(&mut self) -> Vec<GuestNotice> {
        std::mem::take(&mut self.notices)
    }

    pub fn requests_unanswered(&self) -> usize {
        self.requests_unanswered.len()
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
        self.last_tick_ms = now_ms;
        if slot == SlotState::Idle {
            // An idle slot means the replayed handler ended.
            self.tape.end();
        }
        self.partner_calls_queued
            .retain(|(_, at)| now_ms.saturating_sub(*at) <= PARTNER_CALL_POP_EXPECTED_MS);
        if !self.is_active_in(local_phase) {
            return None;
        }
        self.request_player_pops_held_too_long(now_ms);
        if self.restart_wait_goes_on(now_ms) {
            return None;
        }
        if !self.earlier_queued.is_empty() {
            // An entry of a window or epoch left behind is still in the engine queue.
            self.leave_when_drained();
            return None;
        }
        let partner_calls_first =
            earlier_partner_calls_waiting || !self.partner_calls_queued.is_empty();
        let releasable = !partner_calls_first || self.in_flight() != InFlight::Idle;
        let entry = if releasable {
            self.release_after_host_entry_end(slot, now_ms)
        } else {
            None
        };
        self.leave_when_drained();
        entry
    }

    /// The host's handler of entry `seq` ended. Marks arrive in seq order on an ordered channel,
    /// after every tape value of that handler, so entries up to `seq` may now be replayed.
    pub fn host_entry_ended(&mut self, phase: u16, epoch: u32, seq: u64) -> bool {
        let Joined::Yes {
            phase: joined_phase,
            epoch: joined_epoch,
            ..
        } = self.joined
        else {
            return false;
        };
        if phase != joined_phase || epoch != joined_epoch {
            return false;
        }
        self.host_entries_ended_through = Some(
            self.host_entries_ended_through
                .map_or(seq, |through| through.max(seq)),
        );
        true
    }

    pub const fn waiting_for_host_entry_end(&self) -> Option<u64> {
        match self.waiting_for_host_entry_end {
            Some((seq, _)) => Some(seq),
            None => None,
        }
    }

    pub fn queued_call_lost(&mut self) -> Option<u64> {
        self.replayer.queued_call_lost()
    }

    /// The entry the last tick returned is in the engine queue in the slot of `arguments`.
    pub fn entry_queued_at(&mut self, arguments: u64) {
        self.replayer.entry_queued_at(arguments);
        self.queued_waited_ms = 0;
    }

    /// An entry of ours is in the engine queue: the queued one, or one of a window or epoch
    /// left behind. Other relay paths queue nothing beside it.
    pub fn has_own_call_queued(&self) -> bool {
        self.replayer.queued_call().is_some() || !self.earlier_queued.is_empty()
    }

    pub fn earlier_calls_queued(&self) -> usize {
        self.earlier_queued.len()
    }

    /// Once per tick, in any role: `still_queued(name, arguments)` reads whether that queue slot
    /// still holds the call. The queued entry goes back to pending once its slot lost it, and is
    /// skipped past the limit; earlier calls are dropped the same way.
    pub fn check_queued_calls(
        &mut self,
        slot: SlotState,
        now_ms: u64,
        mut still_queued: impl FnMut(&str, u64) -> bool,
    ) -> Vec<QueuedCallNotice> {
        let counted_ms = counted_wait_ms(slot, self.last_queue_check_ms, now_ms);
        self.last_queue_check_ms = Some(now_ms);
        let mut notices = Vec::new();
        if let Some(call) = self.replayer.queued_call() {
            self.queued_waited_ms = self.queued_waited_ms.saturating_add(counted_ms);
            if !still_queued(&call.name, call.arguments) {
                self.replayer.queued_call_lost();
                notices.push(QueuedCallNotice::LeftQueue { name: call.name });
            } else if self.queued_waited_ms > QUEUED_CALL_GIVEN_UP_AFTER_MS {
                self.replayer.give_up_queued();
                notices.push(QueuedCallNotice::GaveUp {
                    name: call.name,
                    waited_ms: self.queued_waited_ms,
                });
            }
        }
        notices.extend(self.earlier_queued.check(counted_ms, still_queued));
        notices
    }

    /// Keeps a host tape value of the joined fight's current epoch; anything else is dropped.
    pub fn receive_tape(&mut self, tape: FightTape) -> bool {
        let Joined::Yes { phase, epoch, .. } = self.joined else {
            return false;
        };
        if tape.phase != phase || tape.epoch != epoch || tape.seq < self.replayer.next_seq() {
            return false;
        }
        self.tape.load(vec![TapedResult {
            key: TapeKey {
                seq: tape.seq,
                native: tape.native,
                call_index: tape.call_index,
            },
            value: TapeValue::from_wire(tape.value),
        }]);
        true
    }

    /// A taped native returned `local` on this game. Inside a replayed handler the host's value
    /// for the same read is used; outside one (`None`) the game keeps its own.
    /// `slot` is the script slot as the read happens: an idle slot means the replayed handler
    /// already ended, so the read belongs to no entry.
    pub fn read_taped(
        &mut self,
        native: &str,
        local: TapeValue,
        local_phase: Option<u16>,
        slot: SlotState,
    ) -> Option<GuestTapeRead> {
        if !self.is_active_in(local_phase) {
            return None;
        }
        if slot == SlotState::Idle {
            self.tape.end();
            return None;
        }
        let seq = self.tape.current_seq()?;
        let read = self.tape.read(native, local);
        if !read.from_host {
            return Some(GuestTapeRead {
                value: read.value,
                log: read.log,
            });
        }
        self.tape_hits_logged
            .retain(|(logged_seq, _)| *logged_seq == seq);
        let first_hit = self.tape_hits_logged.insert((seq, native.to_owned()));
        let hit_line = first_hit.then(|| {
            format!(
                "fight tape: {native} in seq {seq} took the host's {:?} (own {local:?})",
                read.value
            )
        });
        Some(GuestTapeRead {
            value: read.value,
            log: read.log.or(hit_line),
        })
    }

    pub const fn tape_seq(&self) -> Option<u64> {
        self.tape.current_seq()
    }

    fn note_ran_outside_window(&mut self, name: &str, verdict: GuestPop, local_phase: Option<u16>) {
        let Some(phase) = local_phase else {
            return;
        };
        if !matches!(self.joined, Joined::No)
            || matches!(verdict, GuestPop::Hold | GuestPop::HeldForHostCopy)
        {
            return;
        }
        if self.ran_outside_window_phase != Some(phase) {
            self.ran_outside_window_phase = Some(phase);
            self.ran_outside_window.clear();
        }
        self.ran_outside_window.insert(name.to_owned());
    }

    fn skips_arming_entry(&mut self, entry: &StreamEntry, epoch: u32) -> bool {
        let is_arming_entry = entry.seq == 0
            && entry.epoch == epoch
            && self.arming_entry_to_skip.as_deref() == Some(entry.name.as_str());
        if !is_arming_entry || !self.replayer.skip_entry(0) {
            return false;
        }
        self.arming_entry_to_skip = None;
        // A later own copy of this name then pairs with this entry, as one the stream already ran.
        self.host_copy_arrived(&entry.name);
        self.notices.push(GuestNotice::ArmingEntrySkipped {
            name: entry.name.clone(),
            epoch,
        });
        true
    }

    fn clear_tape(&mut self) {
        self.tape.clear();
        self.tape_hits_logged.clear();
    }

    fn forget_epoch_marks(&mut self) {
        self.host_entries_ended_through = None;
        self.waiting_for_host_entry_end = None;
        self.held_pops_unmatched.clear();
        self.held_pops_logged.clear();
    }

    fn release_after_host_entry_end(
        &mut self,
        slot: SlotState,
        now_ms: u64,
    ) -> Option<StreamEntry> {
        // A taped native cannot wait inside its handler (its thunk calls the impl once through
        // 0x3EFD80 and pushes the results at once), so the entry itself waits for the host's end.
        // Only an entry that may read a taped native waits; the rest keep the one-in-flight rule.
        let ended_through = self.host_entries_ended_through;
        let waiting = self.waiting_for_host_entry_end;
        let window = self.window();
        let mut blocked = None;
        let mut gave_up = false;
        let entry = self.replayer.tick_when(slot, |entry| {
            let seq = entry.seq;
            if window.is_some_and(|window| !window.entry_waits_for_host_end(&entry.name)) {
                return true;
            }
            if ended_through.is_some_and(|through| through >= seq) {
                return true;
            }
            let since_ms = match waiting {
                Some((waiting_seq, since_ms)) if waiting_seq == seq => since_ms,
                _ => now_ms,
            };
            if now_ms.saturating_sub(since_ms) > HOST_ENTRY_END_WAIT_LIMIT_MS {
                gave_up = true;
                return true;
            }
            blocked = Some((seq, since_ms));
            false
        });
        let Some(entry) = entry else {
            if blocked.is_some() {
                self.waiting_for_host_entry_end = blocked;
            }
            return None;
        };
        if let Some((waiting_seq, since_ms)) = self.waiting_for_host_entry_end.take()
            && waiting_seq == entry.seq
        {
            let waited_ms = now_ms.saturating_sub(since_ms);
            if gave_up {
                self.notices.push(GuestNotice::HostEntryEndNeverCame {
                    seq: entry.seq,
                    waited_ms,
                });
            } else if waited_ms >= HOST_ENTRY_END_WAIT_LOGGED_FROM_MS {
                self.notices.push(GuestNotice::WaitedForHostEntryEnd {
                    seq: entry.seq,
                    waited_ms,
                });
            }
        }
        Some(entry)
    }

    fn own_copy_pairs(&mut self, name: &str) -> bool {
        let own = self.own_copies.entry(name.to_owned()).or_insert(0);
        *own += 1;
        *own <= self.host_copies.get(name).copied().unwrap_or(0)
    }

    fn host_copy_arrived(&mut self, name: &str) {
        let host = self.host_copies.entry(name.to_owned()).or_insert(0);
        *host += 1;
        if *host > self.own_copies.get(name).copied().unwrap_or(0) {
            // No own pop to pair with yet: this copy pairs with the next one.
            return;
        }
        if let Some(index) = self
            .requests_unanswered
            .iter()
            .position(|asked| asked == name)
        {
            self.requests_unanswered.remove(index);
        } else if let Some(index) = self
            .player_pops_held
            .iter()
            .position(|held| held.name == name)
        {
            let held = self.player_pops_held.remove(index);
            self.notices.push(GuestNotice::PlayerPopPairedWithHostCopy {
                name: held.name,
                held_ms: self.last_tick_ms.saturating_sub(held.since_ms),
            });
        } else if let Some(index) = self
            .held_pops_unmatched
            .iter()
            .position(|held| held == name)
        {
            self.held_pops_unmatched.remove(index);
        }
    }

    fn request_player_pops_held_too_long(&mut self, now_ms: u64) {
        let Joined::Yes { phase, epoch, .. } = self.joined else {
            return;
        };
        let (due, kept): (Vec<_>, Vec<_>) = std::mem::take(&mut self.player_pops_held)
            .into_iter()
            .partition(|held| now_ms.saturating_sub(held.since_ms) > OWN_POP_HELD_FOR_HOST_COPY_MS);
        self.player_pops_held = kept;
        for held in due {
            let host_copies_seen = self.host_copies.get(&held.name).copied().unwrap_or(0);
            self.requests_unanswered.push(held.name.clone());
            self.notices.push(GuestNotice::RequestSent {
                name: held.name.clone(),
                host_copies_seen,
            });
            self.requests_out.push(FightRequest {
                phase,
                epoch,
                kind: FightRequestKind::RunPop {
                    name: held.name,
                    args: held.args,
                    host_copies_seen,
                },
            });
        }
    }

    fn forget_copies(&mut self) {
        self.requests_unanswered.clear();
        self.host_copies.clear();
        self.own_copies.clear();
        self.player_pops_held.clear();
    }

    fn note_held_pops_never_matched(&mut self, held: Vec<String>, epoch: u32) {
        for name in held {
            if self.held_pops_logged.insert(name.clone()) {
                self.notices
                    .push(GuestNotice::HeldPopNeverMatched { name, epoch });
            }
        }
    }

    fn restart_wait_goes_on(&mut self, now_ms: u64) -> bool {
        let Joined::Yes { phase, epoch, .. } = self.joined else {
            return false;
        };
        match self.restart_wait {
            RestartWait::None => false,
            RestartWait::ForOwnRestart { since_ms } => {
                if now_ms.saturating_sub(since_ms) <= PARTNER_RESTART_EXPECTED_MS {
                    return true;
                }
                self.restart_wait = RestartWait::None;
                self.notices.push(GuestNotice::HostRestartedAlone { epoch });
                false
            }
            RestartWait::ForHost {
                since_ms,
                request_sent,
            } => {
                let waited = now_ms.saturating_sub(since_ms);
                if !request_sent && waited > PARTNER_RESTART_EXPECTED_MS {
                    self.requests_out.push(FightRequest {
                        phase,
                        epoch,
                        kind: FightRequestKind::RestartPhase,
                    });
                    self.restart_wait = RestartWait::ForHost {
                        since_ms,
                        request_sent: true,
                    };
                    self.notices.push(GuestNotice::RestartRequested { epoch });
                } else if request_sent && waited > 2 * PARTNER_RESTART_EXPECTED_MS {
                    let dropped = self.leave();
                    self.notices
                        .push(GuestNotice::LeftWithoutHostRestart { dropped });
                }
                true
            }
        }
    }

    fn keep_forgotten_calls(&mut self) {
        for mut call in self.replayer.take_forgotten() {
            call.waited_ms = self.queued_waited_ms;
            self.earlier_queued.add(call);
        }
    }

    const fn is_draining(&self) -> bool {
        matches!(
            self.joined,
            Joined::Yes {
                end_seq: Some(_),
                ..
            }
        )
    }

    fn next_window_wait(&self, message: &Message, local_phase: Option<u16>) -> Option<HeldUntil> {
        let Message::FightWindow(mark) = message else {
            return None;
        };
        match mark.change {
            FightWindowChange::Opened { .. } if self.is_draining() => {
                Some(HeldUntil::EarlierWindowDrained)
            }
            FightWindowChange::Restarted => {
                let Joined::Yes { phase, window, .. } = self.joined else {
                    return None;
                };
                let same_window = (window.number, window.part) == (mark.number, mark.part);
                (same_window && mark.phase != phase && local_phase != Some(mark.phase))
                    .then_some(HeldUntil::Phase(mark.phase))
            }
            FightWindowChange::Opened { .. }
            | FightWindowChange::Closed { .. }
            | FightWindowChange::EntryEnded { .. } => None,
        }
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
