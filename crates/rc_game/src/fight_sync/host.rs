use rc_proto::{
    FightRequest, FightRequestKind, FightWindowChange, FightWindowMark, Message, StoryCallArg,
};

use crate::fight_sync::{
    FightWindow, FightWindowData, HostPop, HostRequests, PopSource, RequestCall, RequestReceipt,
    SlotState, StreamEntry, StreamRecorder, WindowCloseCause, WindowEvent, WindowState,
    fight_windows_of_pack,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostStreamOut {
    /// A start id ran; the hook notes how many callbacks are registered now.
    Armed(&'static FightWindowData),
    Opened {
        phase: u16,
        epoch: u32,
        window: &'static FightWindowData,
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
}

impl HostStreamOut {
    /// What the guest is sent for this step; arming stays on the host.
    pub fn message(&self) -> Option<Message> {
        match self {
            Self::Armed(_) => None,
            Self::Opened {
                phase,
                epoch,
                window,
            } => Some(Message::FightWindow(FightWindowMark {
                phase: *phase,
                epoch: *epoch,
                number: window.number,
                part: window.part,
                change: FightWindowChange::Opened,
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

    pub fn enter_phase(&mut self, phase: u16, pack: &str) -> Vec<HostStreamOut> {
        if self.phase == Some(phase) && self.window.pack() == pack {
            return Vec::new();
        }
        let closed = self.window.enter_pack(pack);
        let outs = self.outs_of(closed);
        self.phase = Some(phase);
        self.recorder.enter_phase(phase);
        self.fewest_registered_since_armed = None;
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
            self.requests.clear();
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

    pub fn receive_request(&mut self, request: FightRequest, now_ms: u64) -> HostRequestOut {
        if self.phase != Some(request.phase) || self.window.open_window().is_none() {
            return HostRequestOut::NotInWindow;
        }
        if request.epoch != self.recorder.epoch() {
            return HostRequestOut::Stale;
        }
        match request.kind {
            FightRequestKind::RunPop { name, args } => {
                HostRequestOut::Run(self.requests.receive(RequestCall { name, args }, now_ms))
            }
            FightRequestKind::RestartPhase => HostRequestOut::RestartPhase,
        }
    }

    /// A call is about to pop on the host's slot inside the window: a queued request runs once,
    /// and the host's own copy of a call it ran for a request close to it is held.
    pub fn judge_pop(&mut self, name: &str, source: PopSource, now_ms: u64) -> HostPop {
        if self.window.open_window().is_none() {
            return HostPop::Run;
        }
        self.requests.pop(name, source, now_ms)
    }

    /// Once per host tick: the guest's request to queue now, one at a time.
    pub fn tick_requests(
        &mut self,
        slot: SlotState,
        other_calls_waiting: bool,
        now_ms: u64,
    ) -> Option<RequestCall> {
        self.window.open_window()?;
        self.requests.tick(slot, other_calls_waiting, now_ms)
    }

    pub fn request_call_lost(&mut self) -> Option<String> {
        self.requests.queued_call_lost()
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
    /// player-local message), then it may close the window; outside it may arm one.
    pub fn popped(
        &mut self,
        name: &str,
        args: Vec<StoryCallArg>,
        registered_player_local: bool,
    ) -> Vec<HostStreamOut> {
        let mut outs = Vec::new();
        if let Some(window) = self.window.open_window()
            && !registered_player_local
            && !window.is_player_local_handler(name)
        {
            outs.push(HostStreamOut::Entry(self.recorder.record(name, args)));
            self.restarted_with_no_entry_since = false;
        }
        outs.extend(self.function_ran(name));
        outs
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
        if matches!(event, Some(WindowEvent::Armed(_))) {
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
            WindowEvent::Armed(window) => vec![HostStreamOut::Armed(window)],
            WindowEvent::Opened(window) => {
                // A new epoch per window, so the guest replays each window from seq 0.
                let epoch = self.recorder.restart();
                self.requests.clear();
                self.restarted_with_no_entry_since = false;
                vec![HostStreamOut::Opened {
                    phase,
                    epoch,
                    window,
                }]
            }
            WindowEvent::Closed(window, cause) => {
                self.requests.clear();
                vec![HostStreamOut::Closed {
                    phase,
                    epoch: self.recorder.epoch(),
                    window,
                    cause,
                    end_seq: self.recorder.next_seq(),
                }]
            }
        }
    }
}
