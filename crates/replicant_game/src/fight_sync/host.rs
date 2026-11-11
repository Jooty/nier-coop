use std::collections::{HashMap, HashSet};

use replicant_proto::{
    FightRadioWaitEnded, FightRequest, FightRequestAnswer, FightRequestKind, FightRequestRefusal,
    FightTape, FightWindowChange, FightWindowMark, Message, StoryCallArg,
};

use crate::fight_sync::{
    EarlierQueuedCalls, FightWindow, FightWindowData, HOST_TAPE_LOGGED_KEYS_KEPT,
    HOST_TAPE_LOGGED_SEQS_BACK, HostPop, HostRadioWaits, HostRequests, HostTape, PopSource,
    QUEUED_CALL_GIVEN_UP_AFTER_MS, QueueRefusal, QueuedCallNotice, RadioWaitPoll, RadioWaitRun,
    RequestAnswer, RequestCall, RequestReceipt, SceneHoldChange, SlotState, StreamEntry,
    StreamRecorder, TapeValue, WindowArming, WindowCloseCause, WindowEvent, WindowState,
    counted_wait_ms, fight_windows_of_pack,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostStreamOut {
    /// A start id ran (the hook notes how many callbacks are registered now), or an arm id
    /// popped and `Opened` follows at once, then that pop as entry 0. A start id that popped is
    /// entry 0 too, sent when its window opens.
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct PopRequestOut {
    out: HostRequestOut,
    refused: Option<RequestAnswer>,
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
    answers_out: Vec<FightRequestAnswer>,
    radio_waits: HostRadioWaits,
    start_pop: Option<(&'static str, Vec<StoryCallArg>)>,
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
            answers_out: Vec::new(),
            radio_waits: HostRadioWaits::new(),
            start_pop: None,
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

    /// `name` is a start id of a window of this pack: its pop may become a window's entry 0.
    pub fn is_start_id(&self, name: &str) -> bool {
        fight_windows_of_pack(self.window.pack()).any(|row| row.start_ids.contains(&name))
    }

    /// A new phase of another pack closes the window. A new phase of the same pack keeps an
    /// open window and starts a new epoch in it, which the guest is told as a restart.
    pub fn enter_phase(&mut self, phase: u16, pack: &str) -> Vec<HostStreamOut> {
        if self.phase == Some(phase) && self.window.pack() == pack {
            return Vec::new();
        }
        let closed = self.window.enter_pack(pack);
        let mut outs = self.outs_of(closed);
        let stays_open = self.window.open_window().is_some();
        if stays_open {
            // Answered in the phase and epoch they were asked in, so before both move on.
            self.forget_requests();
        }
        self.phase = Some(phase);
        let epoch = self.recorder.enter_phase(phase);
        self.fewest_registered_since_armed = None;
        if let Some(window) = self.window.open_window() {
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
            self.forget_requests();
            let epoch = self.recorder.restart();
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

    /// A guest request. A pop is kept only for a function of the open window or a name streamed in
    /// it, under the cap, while this epoch's stream carried fewer copies than the one asked for.
    /// Each pop request gets one answer: now when it is not kept, else once it ran or was dropped.
    pub fn receive_request(&mut self, request: FightRequest) -> HostRequestOut {
        let FightRequest { phase, epoch, kind } = request;
        let (name, args, copy_number) = match kind {
            FightRequestKind::RunPop {
                name,
                args,
                copy_number,
            } => (name, args, copy_number),
            FightRequestKind::RestartPhase => {
                return self.restart_request_out(phase, epoch);
            }
        };
        let out = self.pop_request_out(
            phase,
            epoch,
            RequestCall {
                name,
                args,
                copy_number,
            },
        );
        if let Some(refused) = out.refused {
            self.answers_out.push(FightRequestAnswer {
                phase,
                epoch,
                name: refused.name,
                copy_number: refused.copy_number,
                outcome: refused.outcome,
            });
        }
        out.out
    }

    /// A call is about to pop on the host's slot inside the window: the queued request runs once
    /// when it pops from the slot it took; every other pop is the host's own and runs.
    /// `arguments` is the popped slot's argument block.
    pub fn judge_pop(&mut self, name: &str, source: PopSource, arguments: u64) -> HostPop {
        if self.window.open_window().is_none() {
            return HostPop::Run;
        }
        let streamed = self.streamed_copies(name);
        let pop = self.requests.pop(name, source, arguments, streamed);
        self.collect_answers();
        pop
    }

    /// The answers to the guest's requests since the last call, to send to the guest.
    pub fn take_answers(&mut self) -> Vec<FightRequestAnswer> {
        std::mem::take(&mut self.answers_out)
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

    /// Once per host tick: the guest's request to queue now, one at a time. None is queued while
    /// `own_scene_playing`, the host's own event scene, lasts.
    pub fn tick_requests(
        &mut self,
        slot: SlotState,
        other_calls_waiting: bool,
        own_scene_playing: bool,
    ) -> Option<RequestCall> {
        self.window.open_window()?;
        if !self.earlier_queued.is_empty() {
            return None;
        }
        let streamed = &self.streamed_this_epoch;
        let call = self
            .requests
            .tick(slot, other_calls_waiting, own_scene_playing, |name| {
                streamed.get(name).copied().unwrap_or(0)
            });
        self.collect_answers();
        call
    }

    pub fn take_request_scene_hold_change(&mut self) -> Option<SceneHoldChange> {
        self.requests.take_scene_hold_change()
    }

    pub fn request_call_lost(&mut self) -> Option<String> {
        self.requests.queued_call_lost()
    }

    pub fn request_queue_refused(&mut self) -> Option<QueueRefusal> {
        let refusal = self.requests.queue_refused();
        self.collect_answers();
        refusal
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
                self.collect_answers();
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
    /// pop opens the window and is streamed as its entry 0. A start id's pop is kept and streamed
    /// as entry 0 once its window opens, so the guest's game sets the fight up before the rest.
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
        let armed = self.function_ran(name);
        let armed_by_this_pop = armed.iter().find_map(|out| match out {
            HostStreamOut::Armed {
                window,
                by,
                arming: WindowArming::StartId,
            } if !registered_player_local && !window.is_player_local_handler(name) => Some(*by),
            _ => None,
        });
        if let Some(start_id) = armed_by_this_pop {
            self.start_pop = Some((start_id, args.clone()));
        }
        outs.extend(armed);
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

    /// One call of the game's radio wait in the host's script slot (`starts` in the call that
    /// begins it, `still_waiting` what the game returned). When a wait of a streamed handler of
    /// the open window ends, the guest is sent which one and how long it took.
    pub fn radio_wait_polled(
        &mut self,
        starts: bool,
        still_waiting: bool,
        now_ms: u64,
    ) -> Option<Message> {
        let streamed = self
            .phase
            .filter(|_| self.window.open_window().is_some())
            .zip(self.tape.current_seq());
        let run = streamed.map_or(RadioWaitRun::Other, |(_, seq)| RadioWaitRun::Entry {
            epoch: self.recorder.epoch(),
            seq,
        });
        let mark = self.radio_waits.polled(RadioWaitPoll {
            run,
            starts,
            still_waiting,
            now_ms,
        })?;
        let (phase, _) = streamed?;
        Some(Message::FightRadioWait(FightRadioWaitEnded {
            phase,
            epoch: mark.epoch,
            seq: mark.seq,
            wait_index: mark.wait_index,
            host_waited_ms: u32::try_from(mark.host_waited_ms).unwrap_or(u32::MAX),
        }))
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
            // Only `popped` knows the start was a pop; a traced start has no pop to stream.
            self.start_pop = None;
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
            WindowEvent::Opened(window) => {
                let Some((start_id, args)) = self.start_pop.take() else {
                    return vec![self.opened(phase, window, None)];
                };
                let opened = self.opened(phase, window, Some(start_id));
                vec![opened, self.start_entry(start_id, args)]
            }
            WindowEvent::Disarmed(..) => {
                self.fewest_registered_since_armed = None;
                self.start_pop = None;
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
        self.forget_requests();
        // A new epoch per window, so the guest replays each window from seq 0.
        let epoch = self.recorder.restart();
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

    fn start_entry(&mut self, start_id: &str, args: Vec<StoryCallArg>) -> HostStreamOut {
        // No tape: the handler began before the window opened, so its reads are not numbered.
        let entry = self.recorder.record(start_id, args);
        self.streamed_names.insert(start_id.to_owned());
        *self
            .streamed_this_epoch
            .entry(start_id.to_owned())
            .or_insert(0) += 1;
        HostStreamOut::Entry(entry)
    }

    fn forget_requests(&mut self) {
        // A request already in the engine queue stays there; it blocks the next one until it
        // pops or its slot no longer holds it.
        if let Some(call) = self.requests.queued_call() {
            self.earlier_queued.add(call);
        }
        self.requests.forget();
        self.collect_answers();
        self.streamed_this_epoch.clear();
    }

    fn collect_answers(&mut self) {
        let phase = self.phase.unwrap_or_default();
        let epoch = self.recorder.epoch();
        self.answers_out
            .extend(
                self.requests
                    .take_answers()
                    .into_iter()
                    .map(|answer| FightRequestAnswer {
                        phase,
                        epoch,
                        name: answer.name,
                        copy_number: answer.copy_number,
                        outcome: answer.outcome,
                    }),
            );
    }

    fn restart_request_out(&self, phase: u16, epoch: u32) -> HostRequestOut {
        if self.phase != Some(phase) || self.window.open_window().is_none() {
            return HostRequestOut::NotInWindow;
        }
        if epoch != self.recorder.epoch() {
            return HostRequestOut::Stale;
        }
        HostRequestOut::RestartPhase
    }

    fn pop_request_out(&mut self, phase: u16, epoch: u32, call: RequestCall) -> PopRequestOut {
        let moved_on = |out| PopRequestOut {
            out,
            refused: Some(RequestAnswer::refused(
                &call,
                FightRequestRefusal::FightMovedOn,
            )),
        };
        if self.phase != Some(phase) {
            return moved_on(HostRequestOut::NotInWindow);
        }
        let Some(window) = self.window.open_window() else {
            return moved_on(HostRequestOut::NotInWindow);
        };
        if epoch != self.recorder.epoch() {
            return moved_on(HostRequestOut::Stale);
        }
        let receipt = if !window.window_functions.contains(&call.name.as_str())
            && !self.streamed_names.contains(&call.name)
        {
            RequestReceipt::NotAWindowFunction
        } else if self.streamed_copies(&call.name) >= call.copy_number {
            RequestReceipt::AlreadyRanHere
        } else {
            let answer_if_refused = RequestAnswer::refused(&call, FightRequestRefusal::Full);
            let receipt = self.requests.receive(call);
            return PopRequestOut {
                out: HostRequestOut::Run(receipt),
                refused: receipt.refusal().map(|_| answer_if_refused),
            };
        };
        PopRequestOut {
            out: HostRequestOut::Run(receipt),
            refused: receipt
                .refusal()
                .map(|reason| RequestAnswer::refused(&call, reason)),
        }
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
