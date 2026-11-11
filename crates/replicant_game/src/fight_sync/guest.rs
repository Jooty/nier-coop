use std::collections::{HashMap, HashSet};

use replicant_proto::{
    FightRadioWaitEnded, FightRequest, FightRequestAnswer, FightRequestKind, FightRequestOutcome,
    FightRequestRefusal, FightTape, FightWindowChange, Message, StoryCallArg,
};

use crate::fight_sync::{
    Arrival, EarlierQueuedCalls, FIGHT_WINDOWS, FightWindowData, GuestRadioWaits, GuestTape,
    HOST_ENTRY_END_WAIT_LIMIT_MS, HOST_ENTRY_END_WAIT_LOGGED_FROM_MS, InFlight,
    MAX_HELD_NEXT_WINDOW_MESSAGES, NEXT_WINDOW_HELD_AT_MOST_MS, ONE_RESTART_SIGNALS_SPAN_MS,
    OWN_POP_HELD_FOR_HOST_COPY_MS, OwnQueuedCall, PARTNER_CALL_POP_EXPECTED_MS,
    PARTNER_RESTART_EXPECTED_MS, PopSource, PopVerdict, QUEUED_CALL_GIVEN_UP_AFTER_MS,
    QueuedCallNotice, REFUSED_REQUEST_RESEND_AFTER_MS, REFUSED_REQUEST_RESENDS_BEFORE_RUNNING_HERE,
    RadioWaitContext, RadioWaitMark, RadioWaitPoll, RadioWaitRelease, RadioWaitRun, Replayer,
    RequestCall, SlotState, StartEntry, StartStreamPop, StreamEntry, StreamStallWatch, TapeKey,
    TapeValue, TapedResult, counted_wait_ms,
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
    /// This game's player pop that the host refused to run (not a function of its fight, or it
    /// could not run it after the resends), queued again here: it runs as this player's own.
    RunRefusedHere,
    /// The host's start handler (entry 0 of a window a start id armed) runs here from the stream.
    RunStartEntry,
    /// A copy of the host's start handler that pops after the start already ran here once in
    /// this window: not run again.
    StartAlreadyRan,
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
    /// The host's radio wait `wait_index` of entry `seq` ended; `kept` as for a tape value.
    RadioWaitEnded {
        seq: u64,
        wait_index: u32,
        kept: bool,
    },
    /// The host's answer to a request of this game; `kept` is false when it is for a fight or
    /// epoch this game is not in, or for a request no longer waiting.
    RequestAnswer {
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
    RestartRequested {
        epoch: u32,
    },
    /// The host's restart was not followed by one here in time; the replay goes on.
    HostRestartedAlone {
        epoch: u32,
    },
    /// The host never restarted after the request; this game left the fight's stream.
    LeftWithoutHostRestart {
        dropped: usize,
    },
    /// Entry `seq` waited `waited_ms` for the host's handler of it to end before it was queued.
    WaitedForHostEntryEnd {
        seq: u64,
        waited_ms: u64,
    },
    /// The host's end of entry `seq` never came within the wait limit; it was queued anyway, so a
    /// tape value the host reads later is not used here.
    HostEntryEndNeverCame {
        seq: u64,
        waited_ms: u64,
    },
    /// This game's own pop `name` was held inside the window and the host's stream never carried
    /// a copy to pair with it in the epoch, nor was it sent to the host (logged once per name
    /// and epoch, when the epoch ends).
    HeldPopNeverMatched {
        name: String,
        epoch: u32,
    },
    /// This game's player pop `name` waited `held_ms` and the host's copy of it came: paired,
    /// no request.
    PlayerPopPairedWithHostCopy {
        name: String,
        held_ms: u64,
    },
    /// No host copy of this game's player pop `name` came in time: sent to the host as a
    /// request for its own copy `copy_number` of the epoch.
    RequestSent {
        name: String,
        copy_number: u32,
    },
    RequestRan {
        name: String,
        copy_number: u32,
    },
    /// The host refused the request, and `then` says what this game does about it.
    RequestRefused {
        name: String,
        copy_number: u32,
        reason: FightRequestRefusal,
        then: RefusedRequestStep,
    },
    /// A refused request was sent to the host again (`resend` counts from 1).
    RequestResent {
        name: String,
        copy_number: u32,
        resend: u32,
    },
    /// The host's entry 0 `name` armed its window; this game's own copy had already run, so the
    /// entry was skipped.
    ArmingEntrySkipped {
        name: String,
        epoch: u32,
    },
    /// The host's window opened with its start handler `name` as entry 0. It runs here once,
    /// before the fight's other entries; `ran_here_already` when this game's own copy ran first.
    StartEntryExpected {
        name: String,
        ran_here_already: bool,
    },
    /// The host's next window waited this long and this game still could not follow it: the
    /// earlier window is left undrained, or the host's phase was never reached.
    NextWindowWaitedTooLong {
        waited_ms: u64,
    },
    /// More messages of the host's next window came than are kept while it waits.
    NextWindowMessagesDropped {
        dropped: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefusedRequestStep {
    /// Queued again on this game, where it runs as this player's own.
    RunsHere,
    /// Sent again after `REFUSED_REQUEST_RESEND_AFTER_MS`.
    ResendsSoon,
    /// Nothing: the host's stream already answered it, or its fight moved on.
    Dropped,
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
    copy_number: u32,
    since_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
struct RequestAsked {
    call: RequestCall,
    resends: u32,
    resend_at_ms: Option<u64>,
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

/// The guest's radio wait in the script slot, as the hook saw one call of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestRadioWaitCall {
    pub starts: bool,
    pub still_waiting: bool,
    pub host_in_scene: bool,
    pub here_in_scene: bool,
    pub line_advancing: bool,
    pub now_ms: u64,
}

/// The guest's side of one host fight window: it holds its own pops of the window's phase and
/// runs the host's entries one at a time in seq order, after any earlier partner calls.
#[derive(Debug, Clone)]
pub struct GuestFightStream {
    joined: Joined,
    replayer: Replayer,
    partner_calls_queued: Vec<(String, u64)>,
    requests_out: Vec<FightRequest>,
    requests_unanswered: Vec<RequestAsked>,
    refused_to_run_here: Vec<RequestCall>,
    refused_queued_here: EarlierQueuedCalls,
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
    start_entry: StartEntry,
    popped_and_ran_outside_window: HashSet<String>,
    own_scene_playing: bool,
    earlier_queued: EarlierQueuedCalls,
    queued_waited_ms: u64,
    last_queue_check_ms: Option<u64>,
    next_window: Option<HeldNextWindow>,
    radio_waits: GuestRadioWaits,
    stall: StreamStallWatch,
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
            refused_to_run_here: Vec::new(),
            refused_queued_here: EarlierQueuedCalls::new(),
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
            start_entry: StartEntry::new(),
            popped_and_ran_outside_window: HashSet::new(),
            own_scene_playing: false,
            earlier_queued: EarlierQueuedCalls::new(),
            queued_waited_ms: 0,
            last_queue_check_ms: None,
            next_window: None,
            radio_waits: GuestRadioWaits::new(),
            stall: StreamStallWatch::new(),
        }
    }

    /// A fight message from the host. One that opens the host's next window while this game
    /// still drains the closed one, or that moves the window to a phase this game has not
    /// reached yet, is held with every message after it until `release_next_window` lets it go.
    pub fn receive_message(&mut self, message: Message, local_phase: Option<u16>) -> GuestReceived {
        if let Message::FightRequestAnswer(answer) = message {
            // Answers are of this game's own requests, so they never wait with the next window.
            return GuestReceived::RequestAnswer {
                kept: self.request_answered(answer),
            };
        }
        if !matches!(
            message,
            Message::FightWindow(_)
                | Message::FightStream(_)
                | Message::FightTape(_)
                | Message::FightRadioWait(_)
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
            Message::FightRadioWait(ended) => GuestReceived::RadioWaitEnded {
                seq: ended.seq,
                wait_index: ended.wait_index,
                kept: self.receive_radio_wait_end(ended),
            },
            _ => GuestReceived::NotAFightMessage,
        }
    }

    /// `armed_by`: the pop that is the host window's entry 0. An arm id's pop is skipped when this
    /// game already ran its own copy since its last window in this phase. A start id's pop is the
    /// fight's start handler: it runs here exactly once (see `StartEntry`).
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
        if let Some(start_id) = armed_by.filter(|name| window.start_ids.contains(name)) {
            let ran_here_already = self.ran_outside_window_phase == Some(phase)
                && self.popped_and_ran_outside_window.contains(start_id);
            self.start_entry.expect(start_id, ran_here_already);
            self.notices.push(GuestNotice::StartEntryExpected {
                name: start_id.to_owned(),
                ran_here_already,
            });
            return GuestOpen::Joined(window);
        }
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
        self.start_entry.forget();
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
        self.popped_and_ran_outside_window.clear();
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
        self.start_entry.forget();
        self.keep_forgotten_calls();
        self.clear_tape();
        self.requests_unanswered.clear();
        self.refused_to_run_here.clear();
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
            self.popped_and_ran_outside_window.clear();
        }
        self.forget_epoch_marks();
        self.radio_waits.forget_marks();
        self.stall = StreamStallWatch::new();
        self.joined = Joined::No;
        self.replayer = Replayer::new(Vec::<String>::new());
        self.requests_out.clear();
        self.forget_copies();
        self.arming_entry_to_skip = None;
        self.start_entry.forget();
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
        if source == PopSource::Queued && self.refused_queued_here.popped(name, arguments) {
            return GuestPop::RunRefusedHere;
        }
        if source == PopSource::Queued
            && let Some(index) = self
                .partner_calls_queued
                .iter()
                .position(|(queued, _)| queued == name)
            && !self.replayer.is_queued_at(name, arguments)
        {
            self.partner_calls_queued.remove(index);
            if self.is_active_in(local_phase) && self.start_entry.holds_other_copies_of(name) {
                return GuestPop::StartAlreadyRan;
            }
            return GuestPop::RunPartnerCall;
        }
        if !self.is_active_in(local_phase) {
            return GuestPop::Outside;
        }
        let verdict = self.replayer.pop(name, source, arguments);
        match verdict {
            PopVerdict::RunReplayed { seq: 0 } if self.start_entry.is_expected(name) => {
                // The host's handler began before its window opened: no tape for this entry.
                match self.start_entry.stream_copy_popped(name) {
                    Some(StartStreamPop::Run) => GuestPop::RunStartEntry,
                    Some(StartStreamPop::AlreadyRan) | None => GuestPop::StartAlreadyRan,
                }
            }
            PopVerdict::RunReplayed { seq } => {
                self.tape.begin(seq);
                GuestPop::RunReplayed { seq }
            }
            PopVerdict::RunPlayerLocal => GuestPop::RunPlayerLocal,
            PopVerdict::Hold if registered_player_local => GuestPop::RunPlayerLocal,
            PopVerdict::Hold => {
                if self.count_own_copy(name) > self.host_copies_of(name) {
                    self.held_pops_unmatched.push(name.to_owned());
                }
                GuestPop::Hold
            }
        }
    }

    /// A pop fired by this game's own player (a trap, an action target, a player-move callback).
    /// Host copy N of a name pairs with own pop N of it, whenever either came; every touch counts.
    /// An own pop with no host copy yet is held until one pairs or the hold makes it a request.
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
        let copy_number = self.count_own_copy(name);
        if copy_number <= self.host_copies_of(name) {
            return GuestPop::Hold;
        }
        self.player_pops_held.push(PlayerPopHeld {
            name: name.to_owned(),
            args,
            copy_number,
            since_ms: now_ms,
        });
        GuestPop::HeldForHostCopy
    }

    /// A pop of `name` that is not the stream's really ran on this game (no gate held it), or the
    /// function was traced running. Outside a window it is remembered for the next window's start
    /// entry; inside one, a start entry that still waits is not run a second time.
    pub fn ran_here(&mut self, name: &str, local_phase: Option<u16>) {
        let Some(phase) = local_phase else {
            return;
        };
        let Joined::Yes { epoch, .. } = self.joined else {
            if self.ran_outside_window_phase != Some(phase) {
                self.ran_outside_window_phase = Some(phase);
                self.ran_outside_window.clear();
                self.popped_and_ran_outside_window.clear();
            }
            self.popped_and_ran_outside_window.insert(name.to_owned());
            return;
        };
        if !self.is_active_in(local_phase) || !self.start_entry.another_copy_ran(name) {
            return;
        }
        if self.replayer.skip_pending_entry(0) {
            self.start_entry.stream_copy_skipped();
            self.notices.push(GuestNotice::ArmingEntrySkipped {
                name: name.to_owned(),
                epoch,
            });
        }
    }

    /// Whether this game's own event scene plays right now; the start entry waits for its end.
    pub fn set_own_scene_playing(&mut self, playing: bool) {
        self.own_scene_playing = playing;
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

    /// Once per guest tick while nothing else of ours is queued: a refused player pop to queue
    /// here, where it runs as this player's own.
    pub fn refused_pop_to_run_here(&mut self, slot: SlotState) -> Option<RequestCall> {
        if slot != SlotState::Idle
            || self.has_own_call_queued()
            || !self.refused_queued_here.is_empty()
            || self.refused_to_run_here.is_empty()
        {
            return None;
        }
        Some(self.refused_to_run_here.remove(0))
    }

    /// The call the last `refused_pop_to_run_here` returned is in the engine queue in the slot of
    /// `arguments`, or (`None`) the queue refused it and it is tried again next tick.
    pub fn refused_pop_queued_at(&mut self, call: RequestCall, arguments: Option<u64>) {
        match arguments {
            Some(arguments) => self
                .refused_queued_here
                .add(OwnQueuedCall::new(&call.name, arguments)),
            None => self.refused_to_run_here.insert(0, call),
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
        self.last_tick_ms = now_ms;
        if slot == SlotState::Idle {
            // An idle slot means the replayed handler ended.
            self.tape.end();
        }
        self.partner_calls_queued
            .retain(|(_, at)| now_ms.saturating_sub(*at) <= PARTNER_CALL_POP_EXPECTED_MS);
        if !self.is_active_in(local_phase) {
            self.stall.observe(None, now_ms);
            return None;
        }
        self.stall
            .observe(self.next_waiting_entry().map(|(seq, _)| seq), now_ms);
        self.request_player_pops_held_too_long(now_ms);
        self.resend_refused_requests(now_ms);
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

    /// The next host entry this game has not run: the one in the engine queue, else the lowest
    /// pending one.
    pub fn next_waiting_entry(&self) -> Option<(u64, &str)> {
        if let InFlight::Queued { seq } = self.replayer.in_flight()
            && let Some(name) = self.replayer.in_flight_name()
        {
            return Some((seq, name));
        }
        self.replayer.first_pending()
    }

    /// Once per tick after `tick`: the next host entry when it has waited past the stall limit,
    /// once per `STREAM_STALL_LOGGED_EVERY_MS`, with its name and wait.
    pub fn stream_stall(&mut self, now_ms: u64) -> Option<(u64, String, u64)> {
        let (seq, waited_ms) = self.stall.stall_due(now_ms)?;
        let name = self
            .next_waiting_entry()
            .filter(|(next, _)| *next == seq)
            .map_or_else(String::new, |(_, name)| name.to_owned());
        Some((seq, name, waited_ms))
    }

    /// One call of the game's radio wait in this game's script slot. Returns why the wait ends
    /// now, when the host's game says it is over; nothing outside the joined fight.
    pub fn radio_wait_polled(
        &mut self,
        call: GuestRadioWaitCall,
        local_phase: Option<u16>,
    ) -> Option<RadioWaitRelease> {
        if !self.is_active_in(local_phase) {
            return None;
        }
        let Joined::Yes { epoch, .. } = self.joined else {
            return None;
        };
        let run = self
            .tape
            .current_seq()
            .map_or(RadioWaitRun::Other, |seq| RadioWaitRun::Entry {
                epoch,
                seq,
            });
        let next_waiting = self.next_waiting_entry().map(|(seq, _)| seq);
        self.stall.observe(next_waiting, call.now_ms);
        let host_ended_entry = match run {
            RadioWaitRun::Entry { seq, .. } => self
                .host_entries_ended_through
                .is_some_and(|through| through >= seq),
            RadioWaitRun::Other => false,
        };
        let context = RadioWaitContext {
            entry_behind_ms: self.stall.waiting_ms(call.now_ms),
            host_ended_entry,
            host_in_scene: call.host_in_scene,
            here_in_scene: call.here_in_scene,
            line_advancing: call.line_advancing,
        };
        let poll = RadioWaitPoll {
            run,
            starts: call.starts,
            still_waiting: call.still_waiting,
            now_ms: call.now_ms,
        };
        self.radio_waits.polled(poll, context)
    }

    /// How long a later host entry has waited behind this game's script slot, inside the
    /// joined fight only; as `tick` last saw the next waiting entry.
    pub fn entry_behind_ms(&self, local_phase: Option<u16>, now_ms: u64) -> Option<u64> {
        if !self.is_active_in(local_phase) {
            return None;
        }
        self.stall.waiting_ms(now_ms)
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
        notices.extend(self.earlier_queued.check(counted_ms, &mut still_queued));
        notices.extend(
            self.refused_queued_here
                .check(counted_ms, still_queued)
                .into_iter()
                .map(|notice| match notice {
                    QueuedCallNotice::EarlierGaveUp { name, waited_ms } => {
                        QueuedCallNotice::RefusedPopGaveUp { name, waited_ms }
                    }
                    other => other,
                }),
        );
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
            self.popped_and_ran_outside_window.clear();
        }
        self.ran_outside_window.insert(name.to_owned());
    }

    fn skips_arming_entry(&mut self, entry: &StreamEntry, epoch: u32) -> bool {
        let is_arming_entry = entry.seq == 0
            && entry.epoch == epoch
            && (self.arming_entry_to_skip.as_deref() == Some(entry.name.as_str())
                || self.start_entry.skips_stream_copy_of(&entry.name));
        if !is_arming_entry || !self.replayer.skip_entry(0) {
            return false;
        }
        self.arming_entry_to_skip = None;
        self.start_entry.stream_copy_skipped();
        // A later own copy of this name then pairs with this entry, as one the stream already ran.
        self.host_copy_arrived(&entry.name);
        self.notices.push(GuestNotice::ArmingEntrySkipped {
            name: entry.name.clone(),
            epoch,
        });
        true
    }

    fn receive_radio_wait_end(&mut self, ended: FightRadioWaitEnded) -> bool {
        let Joined::Yes { phase, epoch, .. } = self.joined else {
            return false;
        };
        if ended.phase != phase || ended.epoch != epoch {
            return false;
        }
        self.radio_waits.receive_mark(RadioWaitMark {
            epoch: ended.epoch,
            seq: ended.seq,
            wait_index: ended.wait_index,
            host_waited_ms: u64::from(ended.host_waited_ms),
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
        let start_entry = &self.start_entry;
        let own_scene_playing = self.own_scene_playing;
        let mut blocked = None;
        let mut gave_up = false;
        let entry = self.replayer.tick_when(slot, |entry| {
            let seq = entry.seq;
            if seq == 0 && start_entry.is_expected(&entry.name) {
                // Nothing of the host's start handler is taped, so its end is not waited for.
                return start_entry.may_release(&entry.name, own_scene_playing);
            }
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

    fn count_own_copy(&mut self, name: &str) -> u32 {
        let own = self.own_copies.entry(name.to_owned()).or_insert(0);
        *own += 1;
        *own
    }

    fn host_copies_of(&self, name: &str) -> u32 {
        self.host_copies.get(name).copied().unwrap_or(0)
    }

    fn host_copy_arrived(&mut self, name: &str) {
        let host = self.host_copies.entry(name.to_owned()).or_insert(0);
        *host += 1;
        let copy_number = *host;
        if copy_number > self.own_copies.get(name).copied().unwrap_or(0) {
            // No own pop to pair with yet: this copy pairs with the next one.
            return;
        }
        // Host copy N pairs with own copy N, whatever became of it.
        if let Some(index) = self
            .requests_unanswered
            .iter()
            .position(|asked| asked.call.name == name && asked.call.copy_number == copy_number)
        {
            self.requests_unanswered.remove(index);
        } else if let Some(index) = self
            .player_pops_held
            .iter()
            .position(|held| held.name == name && held.copy_number == copy_number)
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
            self.notices.push(GuestNotice::RequestSent {
                name: held.name.clone(),
                copy_number: held.copy_number,
            });
            let call = RequestCall {
                name: held.name,
                args: held.args,
                copy_number: held.copy_number,
            };
            self.requests_out.push(run_pop_request(phase, epoch, &call));
            self.requests_unanswered.push(RequestAsked {
                call,
                resends: 0,
                resend_at_ms: None,
            });
        }
    }

    fn resend_refused_requests(&mut self, now_ms: u64) {
        let Joined::Yes { phase, epoch, .. } = self.joined else {
            return;
        };
        for asked in &mut self.requests_unanswered {
            if asked.resend_at_ms.is_none_or(|at| now_ms < at) {
                continue;
            }
            asked.resend_at_ms = None;
            asked.resends += 1;
            self.requests_out
                .push(run_pop_request(phase, epoch, &asked.call));
            self.notices.push(GuestNotice::RequestResent {
                name: asked.call.name.clone(),
                copy_number: asked.call.copy_number,
                resend: asked.resends,
            });
        }
    }

    fn request_answered(&mut self, answer: FightRequestAnswer) -> bool {
        let Joined::Yes { phase, epoch, .. } = self.joined else {
            return false;
        };
        if answer.phase != phase || answer.epoch != epoch {
            return false;
        }
        // A request waiting to be resent was answered already; this answer is for nothing here.
        let Some(index) = self.requests_unanswered.iter().position(|asked| {
            asked.resend_at_ms.is_none()
                && asked.call.name == answer.name
                && asked.call.copy_number == answer.copy_number
        }) else {
            return false;
        };
        let FightRequestOutcome::Refused { reason } = answer.outcome else {
            self.requests_unanswered.remove(index);
            self.notices.push(GuestNotice::RequestRan {
                name: answer.name,
                copy_number: answer.copy_number,
            });
            return true;
        };
        let asked = &mut self.requests_unanswered[index];
        let then = match reason {
            FightRequestRefusal::Full | FightRequestRefusal::GaveUp
                if asked.resends < REFUSED_REQUEST_RESENDS_BEFORE_RUNNING_HERE =>
            {
                asked.resend_at_ms = Some(self.last_tick_ms + REFUSED_REQUEST_RESEND_AFTER_MS);
                RefusedRequestStep::ResendsSoon
            }
            FightRequestRefusal::NotAWindowFunction
            | FightRequestRefusal::Full
            | FightRequestRefusal::GaveUp => {
                let asked = self.requests_unanswered.remove(index);
                self.refused_to_run_here.push(asked.call);
                RefusedRequestStep::RunsHere
            }
            FightRequestRefusal::AlreadyStreamed | FightRequestRefusal::FightMovedOn => {
                self.requests_unanswered.remove(index);
                RefusedRequestStep::Dropped
            }
        };
        self.notices.push(GuestNotice::RequestRefused {
            name: answer.name,
            copy_number: answer.copy_number,
            reason,
            then,
        });
        true
    }

    fn forget_copies(&mut self) {
        self.requests_unanswered.clear();
        self.refused_to_run_here.clear();
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

fn run_pop_request(phase: u16, epoch: u32, call: &RequestCall) -> FightRequest {
    FightRequest {
        phase,
        epoch,
        kind: FightRequestKind::RunPop {
            name: call.name.clone(),
            args: call.args.clone(),
            copy_number: call.copy_number,
        },
    }
}
