use std::collections::HashSet;

use rc_proto::{
    FightRequest, FightRequestKind, FightTape, FightWindowChange, Message, StoryCallArg,
};

use crate::fight_sync::{
    Arrival, FIGHT_WINDOWS, FightWindowData, GuestTape, InFlight, ONE_RESTART_SIGNALS_SPAN_MS,
    PARTNER_CALL_POP_EXPECTED_MS, PARTNER_RESTART_EXPECTED_MS, PopSource, PopVerdict,
    REQUEST_ANSWER_EXPECTED_MS, Replayer, SAME_POP_ON_BOTH_GAMES_SPAN_MS, SlotState, StreamEntry,
    TapeKey, TapeValue, TapedResult,
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
    /// A pop only this game's player caused: not run here, sent to the host, which runs it once
    /// and streams it back.
    Requested,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestNotice {
    /// No host restart came in time after this game's own: the host was asked to restart.
    RestartRequested { epoch: u32 },
    /// The host's restart was not followed by one here in time; the replay goes on.
    HostRestartedAlone { epoch: u32 },
    /// The host never restarted after the request; this game left the fight's stream.
    LeftWithoutHostRestart { dropped: usize },
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
    requests_unanswered: Vec<(String, u64)>,
    host_names_seen: Vec<(String, u64)>,
    restart_wait: RestartWait,
    last_own_restart_ms: Option<u64>,
    notices: Vec<GuestNotice>,
    last_tick_ms: u64,
    tape: GuestTape,
    tape_hits_logged: HashSet<(u64, String)>,
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
            host_names_seen: Vec::new(),
            restart_wait: RestartWait::None,
            last_own_restart_ms: None,
            notices: Vec::new(),
            last_tick_ms: 0,
            tape: GuestTape::new(),
            tape_hits_logged: HashSet::new(),
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
            ));
        };
        if joined_phase != phase || (window.number, window.part) != (number, part) {
            return GuestRestart::Joined(self.window_opened(
                phase,
                epoch,
                number,
                part,
                local_phase,
            ));
        }
        if epoch <= joined_epoch {
            return GuestRestart::SameRestart;
        }
        self.replayer.restart(epoch);
        self.clear_tape();
        self.joined = Joined::Yes {
            phase,
            epoch,
            window,
            end_seq: None,
        };
        self.requests_unanswered.clear();
        self.host_names_seen.clear();
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
        self.clear_tape();
        self.requests_unanswered.clear();
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
        let name = entry.name.clone();
        let arrival = self.replayer.receive(entry);
        if arrival == Arrival::Pending {
            self.requests_unanswered.retain(|(asked, _)| *asked != name);
            self.host_names_seen.push((name, self.last_tick_ms));
        }
        GuestArrival::Stream(arrival)
    }

    pub fn leave(&mut self) -> usize {
        let dropped = self.replayer.pending_len();
        self.joined = Joined::No;
        self.replayer = Replayer::new(Vec::<String>::new());
        self.requests_out.clear();
        self.requests_unanswered.clear();
        self.host_names_seen.clear();
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

    /// `registered_player_local`: the hook knows the name as a handler of a player-local message.
    pub fn pop(
        &mut self,
        name: &str,
        source: PopSource,
        local_phase: Option<u16>,
        registered_player_local: bool,
    ) -> GuestPop {
        // The slot runs one handler at a time, so any pop ends the replayed handler's tape.
        self.tape.end();
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
            PopVerdict::RunReplayed { seq } => {
                self.tape.begin(seq);
                GuestPop::RunReplayed { seq }
            }
            PopVerdict::RunPlayerLocal => GuestPop::RunPlayerLocal,
            PopVerdict::Hold if registered_player_local => GuestPop::RunPlayerLocal,
            PopVerdict::Hold => GuestPop::Hold,
        }
    }

    /// A pop fired by this game's own player (a trap, an action target, a player-move callback).
    /// Inside the window it is sent to the host once, unless the host's stream already carries
    /// the same call (both players set it off) or a request for it is still unanswered.
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
            return GuestPop::Outside;
        }
        let Joined::Yes { phase, epoch, .. } = self.joined else {
            return GuestPop::Outside;
        };
        if registered_player_local || self.replayer.is_player_local(name) {
            return GuestPop::RunPlayerLocal;
        }
        if self.restart_wait != RestartWait::None {
            return GuestPop::Hold;
        }
        let in_stream = self.host_names_seen.iter().any(|(seen, at)| {
            seen == name && now_ms.saturating_sub(*at) <= SAME_POP_ON_BOTH_GAMES_SPAN_MS
        });
        let asked = self
            .requests_unanswered
            .iter()
            .any(|(asked, _)| asked == name);
        if in_stream || asked {
            return GuestPop::Hold;
        }
        self.requests_unanswered.push((name.to_owned(), now_ms));
        self.requests_out.push(FightRequest {
            phase,
            epoch,
            kind: FightRequestKind::RunPop {
                name: name.to_owned(),
                args,
            },
        });
        GuestPop::Requested
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
        self.requests_unanswered
            .retain(|(_, at)| now_ms.saturating_sub(*at) <= REQUEST_ANSWER_EXPECTED_MS);
        self.host_names_seen
            .retain(|(_, at)| now_ms.saturating_sub(*at) <= SAME_POP_ON_BOTH_GAMES_SPAN_MS);
        if !self.is_active_in(local_phase) {
            return None;
        }
        if self.restart_wait_goes_on(now_ms) {
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
    pub fn read_taped(
        &mut self,
        native: &str,
        local: TapeValue,
        local_phase: Option<u16>,
    ) -> Option<GuestTapeRead> {
        if !self.is_active_in(local_phase) {
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
        Some(GuestTapeRead {
            value: read.value,
            log: first_hit.then(|| {
                format!(
                    "fight tape: {native} in seq {seq} took the host's {:?} (own {local:?})",
                    read.value
                )
            }),
        })
    }

    pub const fn tape_seq(&self) -> Option<u64> {
        self.tape.current_seq()
    }

    fn clear_tape(&mut self) {
        self.tape.clear();
        self.tape_hits_logged.clear();
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
