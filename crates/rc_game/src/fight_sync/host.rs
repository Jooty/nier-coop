use std::collections::{HashMap, HashSet};

use rc_proto::{
    FightRequest, FightRequestKind, FightTape, FightWindowChange, FightWindowMark, Message,
    StoryCallArg,
};

use crate::fight_sync::{
    EarlierQueuedCalls, FightWindow, FightWindowData, HOST_TAPE_LOGGED_KEYS_KEPT,
    HOST_TAPE_LOGGED_SEQS_BACK, HostPop, HostRequests, HostTape, PopSource,
    QUEUED_CALL_GIVEN_UP_AFTER_MS, QueueRefusal, QueuedCallNotice, RequestCall, RequestReceipt,
    SlotState, StreamEntry, StreamRecorder, TapeValue, WindowArming, WindowCloseCause, WindowEvent,
    WindowState, counted_wait_ms, fight_windows_of_pack,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostStreamOut {
    /// A start id ran (the hook notes how many callbacks are registered now), or an arm id
    /// popped and `Opened` follows at once, then that pop as entry 0.
    Armed {
        window: &'static FightWindowData,
        by: &'static str,
        arming: WindowArming,
    },
    Opened {
        phase: u16,
        epoch: u32,
        window: &'static FightWindowData,
        armed_by: Option<&'static str>,
    },
    Entry(StreamEntry),
    /// The phase restarted inside the open window: a new epoch of the same window.
    Restarted {
        phase: u16,
        epoch: u32,
        window: &'static FightWindowData,
    },
    Closed {
        phase: u16,
        epoch: u32,
        window: &'static FightWindowData,
        cause: WindowCloseCause,
        end_seq: u64,
    },
    /// The handler of entry `seq` ended on the host: every tape value it read is already sent.
    EntryEnded {
        phase: u16,
        epoch: u32,
        window: &'static FightWindowData,
        seq: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostRequestOut {
    Run(RequestReceipt),
    /// The guest restarted alone: the host restarts its own phase so both start a new epoch.
    RestartPhase,
    /// The request names an epoch the host has left (it restarted or the window changed).
    Stale,
    NotInWindow,
}

/// One taped read on the host: the message for the guest, and a log line for the first read of
/// that native in the entry.
#[derive(Debug, Clone, PartialEq)]
pub struct HostTapeOut {
    pub message: Message,
    pub log: Option<String>,
}

/// The host's side: the fight window of its phase pack and the stream of every pop its script
/// slot runs while the window is open. Each window opening starts a new epoch at seq 0.
#[derive(Debug, Clone)]
pub struct HostFightStream {
    phase: Option<u16>,
    window: FightWindow,
    recorder: StreamRecorder,
    fewest_registered_since_armed: Option<usize>,
    requests: HostRequests,
    restarted_with_no_entry_since: bool,
    tape: HostTape,
    tape_logged: HashSet<(u64, String)>,
    streamed_names: HashSet<String>,
    streamed_this_epoch: HashMap<String, u32>,
    earlier_queued: EarlierQueuedCalls,
    last_queue_check_ms: Option<u64>,
}

impl HostStreamOut {
    /// What the guest is sent for this step; arming stays on the host.
    pub fn message(&self) -> Option<Message> {
        match self {
            Self::Armed { .. } => None,
            Self::Opened {
                phase,
                epoch,
                window,
                armed_by,
            } => Some(Message::FightWindow(FightWindowMark {
                phase: *phase,
                epoch: *epoch,
                number: window.number,
                part: window.part,
                change: FightWindowChange::Opened {
                    armed_by: armed_by.map(str::to_owned),
                },
            })),
            Self::Entry(entry) => Some(Message::FightStream(entry.to_wire())),
            Self::Restarted {
                phase,
                epoch,
                window,
            } => Some(Message::FightWindow(FightWindowMark {
                phase: *phase,
                epoch: *epoch,
                number: window.number,
                part: window.part,
                change: FightWindowChange::Restarted,
            })),
            Self::Closed {
                phase,
                epoch,
                window,
                end_seq,
                ..
            } => Some(Message::FightWindow(FightWindowMark {
                phase: *phase,
                epoch: *epoch,
                number: window.number,
                part: window.part,
                change: FightWindowChange::Closed { end_seq: *end_seq },
            })),
            Self::EntryEnded {
                phase,
                epoch,
                window,
                seq,
            } => Some(Message::FightWindow(FightWindowMark {
                phase: *phase,
                epoch: *epoch,
                number: window.number,
                part: window.part,
                change: FightWindowChange::EntryEnded { seq: *seq },
            })),
        }
    }
}

impl Default for HostFightStream {
    fn default() -> Self {
        Self::new()
    }
}

impl HostFightStream {
    pub fn new() -> Self {
        Self {
            phase: None,
            window: FightWindow::new(""),
            recorder: StreamRecorder::new(0),
            fewest_registered_since_armed: None,
            requests: HostRequests::new(),
            restarted_with_no_entry_since: false,
            tape: HostTape::new(),
            tape_logged: HashSet::new(),
            streamed_names: HashSet::new(),
            streamed_this_epoch: HashMap::new(),
            earlier_queued: EarlierQueuedCalls::new(),
            last_queue_check_ms: None,
        }
    }

    pub const fn epoch(&self) -> u32 {
        self.recorder.epoch()
    }

    pub const fn phase(&self) -> Option<u16> {
        self.phase
    }

    pub fn is_open_in(&self, phase: u16) -> bool {
        self.phase == Some(phase) && self.window.open_window().is_some()
    }

    pub const fn is_armed(&self) -> bool {
        matches!(self.window.state(), WindowState::Armed(_))
    }

    pub const fn open_window(&self) -> Option<&'static FightWindowData> {
        self.window.open_window()
    }

    pub fn pack(&self) -> &str {
        self.window.pack()
    }

    /// A new phase of another pack closes the window. A new phase of the same pack keeps an
    /// open window and starts a new epoch in it, which the guest is told as a restart.
    pub fn enter_phase(&mut self, phase: u16, pack: &str) -> Vec<HostStreamOut> {
        if self.phase == Some(phase) && self.window.pack() == pack {
            return Vec::new();
        }
        let closed = self.window.enter_pack(pack);
        let mut outs = self.outs_of(closed);
        self.phase = Some(phase);
        let epoch = self.recorder.enter_phase(phase);
        self.fewest_registered_since_armed = None;
        if let Some(window) = self.window.open_window() {
            self.forget_requests();
            self.tape.end();
            self.tape_logged.clear();
            self.restarted_with_no_entry_since = true;
            outs.push(HostStreamOut::Restarted {
                phase,
                epoch,
                window,
            });
        }
        outs
    }

    /// The phase restarted. Inside an open window the fight goes on at a new epoch (the guest
    /// waits for it); a second restart signal with no entry since is the same restart.
    pub fn restart(&mut self) -> Vec<HostStreamOut> {
        if let Some(window) = self.window.open_window() {
            if self.restarted_with_no_entry_since && self.recorder.next_seq() == 0 {
                return Vec::new();
            }
            let epoch = self.recorder.restart();
            self.forget_requests();
            self.tape.end();
            self.tape_logged.clear();
            self.restarted_with_no_entry_since = true;
            return vec![HostStreamOut::Restarted {
                phase: self.phase.unwrap_or_default(),
                epoch,
                window,
            }];
        }
        let closed = self.window.restart();
        self.fewest_registered_since_armed = None;
        self.outs_of(closed)
    }

    /// A guest request. A pop is kept only when its name is a function of the open window or one
    /// this game already streamed in it, while fewer than the cap wait, and while this epoch's
    /// stream carried no more copies of it than the guest had seen when it asked.
    pub fn receive_request(&mut self, request: FightRequest) -> HostRequestOut {
        if self.phase != Some(request.phase) {
            return HostRequestOut::NotInWindow;
        }
        let Some(window) = self.window.open_window() else {
            return HostRequestOut::NotInWindow;
        };
        if request.epoch != self.recorder.epoch() {
            return HostRequestOut::Stale;
        }
        match request.kind {
            FightRequestKind::RunPop {
                name,
                args,
                host_copies_seen,
            } => {
                if !window.window_functions.contains(&name.as_str())
                    && !self.streamed_names.contains(&name)
                {
                    return HostRequestOut::Run(RequestReceipt::NotAWindowFunction);
                }
                if self.streamed_copies(&name) > host_copies_seen {
                    return HostRequestOut::Run(RequestReceipt::AlreadyRanHere);
                }
                HostRequestOut::Run(self.requests.receive(RequestCall { name, args }))
            }
            FightRequestKind::RestartPhase => HostRequestOut::RestartPhase,
        }
    }

    /// A call is about to pop on the host's slot inside the window: a queued request runs once,
    /// and the host's own copy of a call it ran for a request in this epoch is held, one per run.
    /// `arguments` is the popped slot's argument block.
    pub fn judge_pop(&mut self, name: &str, source: PopSource, arguments: u64) -> HostPop {
        if self.window.open_window().is_none() {
            return HostPop::Run;
        }
        self.requests.pop(name, source, arguments)
    }

    /// How many entries of `name` this epoch's stream carried.
    pub fn streamed_copies(&self, name: &str) -> u32 {
        self.streamed_this_epoch.get(name).copied().unwrap_or(0)
    }

    /// A queued call popped: true when it is a request of a window or epoch left behind, which
    /// then runs as any call of this game.
    pub fn earlier_request_popped(&mut self, name: &str, arguments: u64) -> bool {
        self.earlier_queued.popped(name, arguments)
    }

    /// Once per host tick: the guest's request to queue now, one at a time.
    pub fn tick_requests(
        &mut self,
        slot: SlotState,
        other_calls_waiting: bool,
    ) -> Option<RequestCall> {
        self.window.open_window()?;
        if !self.earlier_queued.is_empty() {
            return None;
        }
        self.requests.tick(slot, other_calls_waiting)
    }

    pub fn request_call_lost(&mut self) -> Option<String> {
        self.requests.queued_call_lost()
    }

    pub fn request_queue_refused(&mut self) -> Option<QueueRefusal> {
        self.requests.queue_refused()
    }

    /// The request the last tick returned is in the engine queue in the slot of `arguments`.
    pub fn request_queued_at(&mut self, arguments: u64) {
        self.requests.queued_at(arguments);
    }

    /// A request of ours is in the engine queue: the queued one, or one of a window or epoch left
    /// behind. Other relay paths queue nothing beside it.
    pub fn has_own_call_queued(&self) -> bool {
        self.requests.queued_call().is_some() || !self.earlier_queued.is_empty()
    }

    pub fn earlier_calls_queued(&self) -> usize {
        self.earlier_queued.len()
    }

    /// Once per tick, in any role: `still_queued(name, arguments)` reads whether that queue slot
    /// still holds the call. The queued request is queued again once its slot lost it, and
    /// dropped past the limit; earlier requests are dropped the same way.
    pub fn check_queued_calls(
        &mut self,
        slot: SlotState,
        now_ms: u64,
        mut still_queued: impl FnMut(&str, u64) -> bool,
    ) -> Vec<QueuedCallNotice> {
        let counted_ms = counted_wait_ms(slot, self.last_queue_check_ms, now_ms);
        self.last_queue_check_ms = Some(now_ms);
        let mut notices = Vec::new();
        if let Some(call) = self.requests.queued_call() {
            let waited_ms = self.requests.count_queued_wait(counted_ms);
            if !still_queued(&call.name, call.arguments) {
                self.requests.queued_call_lost();
                notices.push(QueuedCallNotice::LeftQueue { name: call.name });
            } else if waited_ms > QUEUED_CALL_GIVEN_UP_AFTER_MS {
                self.requests.give_up_queued();
                notices.push(QueuedCallNotice::GaveUp {
                    name: call.name,
                    waited_ms,
                });
            }
        }
        notices.extend(self.earlier_queued.check(counted_ms, still_queued));
        notices
    }

    pub fn is_request_queued(&self) -> bool {
        self.requests.is_queued()
    }

    pub fn requests_waiting(&self) -> usize {
        self.requests.pending_len()
    }

    /// `count` callbacks are registered right now. While armed, a count above the fewest seen
    /// since arming means a registration happened, which opens the window.
    pub fn registered_callbacks(&mut self, count: usize) -> Vec<HostStreamOut> {
        if !self.is_armed() {
            return Vec::new();
        }
        let fewest = self.fewest_registered_since_armed.get_or_insert(count);
        if count <= *fewest {
            *fewest = count;
            return Vec::new();
        }
        self.callback_registered()
    }

    pub fn callback_registered(&mut self) -> Vec<HostStreamOut> {
        let opened = self.window.callback_registered();
        self.outs_of(opened)
    }

    /// A popped call is about to run on the host's slot. Inside the window it is streamed unless
    /// it is player-local (the window's list, or `registered_player_local`: a handler of a
    /// player-local message), then it may close the window; outside it may arm one. An arm id's
    /// pop opens the window and is streamed as its entry 0.
    pub fn popped(
        &mut self,
        name: &str,
        args: Vec<StoryCallArg>,
        registered_player_local: bool,
    ) -> Vec<HostStreamOut> {
        // The slot runs one handler at a time, so this pop ends the previous handler's tape.
        let mut outs: Vec<HostStreamOut> = self.end_entry_tape().into_iter().collect();
        if self.window.open_window().is_some() {
            outs.extend(self.stream_pop(name, args, registered_player_local));
            outs.extend(self.function_ran(name));
            return outs;
        }
        outs.extend(self.function_ran(name));
        outs.extend(self.stream_pop(name, args, registered_player_local));
        outs
    }

    /// Once per host tick: an idle script slot means the streamed handler ended, which the guest
    /// is told so it can replay that entry with every value the handler read.
    pub fn script_slot(&mut self, slot: SlotState) -> Option<HostStreamOut> {
        if slot != SlotState::Idle {
            return None;
        }
        self.end_entry_tape()
    }

    /// A taped native returned `value` on the host's game. Inside a streamed handler of the open
    /// window it is numbered and sent to the guest; anywhere else nothing is sent.
    pub fn taped_read(&mut self, native: &str, value: TapeValue) -> Option<HostTapeOut> {
        self.window.open_window()?;
        let phase = self.phase?;
        let taped = self.tape.record(native, value)?;
        let first_in_entry = self.tape_logged.insert((taped.key.seq, native.to_owned()));
        if first_in_entry && self.tape_logged.len() > HOST_TAPE_LOGGED_KEYS_KEPT {
            self.tape_logged
                .retain(|(seq, _)| *seq + HOST_TAPE_LOGGED_SEQS_BACK >= taped.key.seq);
        }
        let log = first_in_entry.then(|| {
            format!(
                "fight tape: {native} in seq {} returned {value:?} here, sent",
                taped.key.seq
            )
        });
        Some(HostTapeOut {
            message: Message::FightTape(FightTape {
                phase,
                epoch: self.recorder.epoch(),
                seq: taped.key.seq,
                native: taped.key.native,
                call_index: taped.key.call_index,
                value: value.to_wire(),
            }),
            log,
        })
    }

    pub const fn tape_seq(&self) -> Option<u64> {
        self.tape.current_seq()
    }

    /// A game over or continue id inside the open window is part of the fight's retry (both
    /// games restart together), unless it starts another window of the pack.
    pub fn function_ran(&mut self, name: &str) -> Vec<HostStreamOut> {
        if let Some(window) = self.window.open_window()
            && matches!(
                window.close_cause(name),
                Some(WindowCloseCause::GameOver | WindowCloseCause::Continue)
            )
            && !fight_windows_of_pack(self.window.pack())
                .any(|row| row != window && row.start_ids.contains(&name))
        {
            return Vec::new();
        }
        let event = self.window.function_ran(name);
        if matches!(event, Some(WindowEvent::Armed(..))) {
            self.fewest_registered_since_armed = None;
        }
        self.outs_of(event)
    }

    fn outs_of(&mut self, event: Option<WindowEvent>) -> Vec<HostStreamOut> {
        let Some(event) = event else {
            return Vec::new();
        };
        let phase = self.phase.unwrap_or_default();
        match event {
            WindowEvent::Armed(window, by) => vec![HostStreamOut::Armed {
                window,
                by,
                arming: WindowArming::StartId,
            }],
            WindowEvent::OpenedByArmId(window, by) => vec![
                HostStreamOut::Armed {
                    window,
                    by,
                    arming: WindowArming::ArmId,
                },
                self.opened(phase, window, Some(by)),
            ],
            WindowEvent::Opened(window) => vec![self.opened(phase, window, None)],
            WindowEvent::Disarmed(..) => {
                self.fewest_registered_since_armed = None;
                Vec::new()
            }
            WindowEvent::Closed(window, cause) => {
                self.forget_requests();
                // The window closes as its end id pops, so nothing that handler reads is taped.
                let ended = self
                    .tape
                    .current_seq()
                    .map(|seq| HostStreamOut::EntryEnded {
                        phase,
                        epoch: self.recorder.epoch(),
                        window,
                        seq,
                    });
                self.tape.end();
                let mut outs: Vec<HostStreamOut> = ended.into_iter().collect();
                outs.push(HostStreamOut::Closed {
                    phase,
                    epoch: self.recorder.epoch(),
                    window,
                    cause,
                    end_seq: self.recorder.next_seq(),
                });
                outs
            }
        }
    }

    fn opened(
        &mut self,
        phase: u16,
        window: &'static FightWindowData,
        armed_by: Option<&'static str>,
    ) -> HostStreamOut {
        // A new epoch per window, so the guest replays each window from seq 0.
        let epoch = self.recorder.restart();
        self.forget_requests();
        self.streamed_names.clear();
        self.tape.end();
        self.tape_logged.clear();
        self.restarted_with_no_entry_since = false;
        HostStreamOut::Opened {
            phase,
            epoch,
            window,
            armed_by,
        }
    }

    fn stream_pop(
        &mut self,
        name: &str,
        args: Vec<StoryCallArg>,
        registered_player_local: bool,
    ) -> Option<HostStreamOut> {
        let window = self.window.open_window()?;
        if registered_player_local || window.is_player_local_handler(name) {
            return None;
        }
        let entry = self.recorder.record(name, args);
        self.streamed_names.insert(name.to_owned());
        *self.streamed_this_epoch.entry(name.to_owned()).or_insert(0) += 1;
        self.tape.begin(entry.seq);
        self.restarted_with_no_entry_since = false;
        Some(HostStreamOut::Entry(entry))
    }

    fn forget_requests(&mut self) {
        // A request already in the engine queue stays there; it blocks the next one until it
        // pops or its slot no longer holds it.
        if let Some(call) = self.requests.queued_call() {
            self.earlier_queued.add(call);
        }
        self.requests.clear();
        self.streamed_this_epoch.clear();
    }

    fn end_entry_tape(&mut self) -> Option<HostStreamOut> {
        let seq = self.tape.current_seq()?;
        self.tape.end();
        // A tape only begins inside the open window, and a close ends it, so the window is open.
        let window = self.window.open_window()?;
        Some(HostStreamOut::EntryEnded {
            phase: self.phase.unwrap_or_default(),
            epoch: self.recorder.epoch(),
            window,
            seq,
        })
    }
}
