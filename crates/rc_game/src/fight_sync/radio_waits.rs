use crate::fight_sync::{
    MAX_RADIO_WAIT_MARKS_KEPT, RADIO_WAIT_BEHIND_STREAM_GRACE_MS,
    RADIO_WAIT_MARK_SLACK_FOR_AN_ADVANCING_LINE_MS,
};

/// The handler a radio wait runs in: a stream entry of the host's fight, or any other one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadioWaitRun {
    Entry { epoch: u32, seq: u64 },
    Other,
}

/// One call of the game's radio wait (`_StartRadio`, polled each tick by the script step).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RadioWaitPoll {
    pub run: RadioWaitRun,
    /// The game starts this wait in this call (its wait state was still 0).
    pub starts: bool,
    /// What the game's own wait returned: true while its radio still plays.
    pub still_waiting: bool,
    pub now_ms: u64,
}

/// A wait of a stream entry that ended on the host: its index in that entry's handler, from 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RadioWaitMark {
    pub epoch: u32,
    pub seq: u64,
    pub wait_index: u32,
    pub host_waited_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadioWaitRelease {
    /// The host's copy of this wait ended, and this one has waited as long: at once when its
    /// line is stalled, else after the slack for an advancing line.
    Mark { line_advancing: bool },
    /// The host's handler ended, or a later host entry waits behind this one.
    HostMovedOn,
    /// The host is in an event scene and this game is not.
    HostInScene,
}

/// What the guest knows around a poll of its radio wait.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RadioWaitContext {
    /// A later host entry has waited behind the script slot this long.
    pub entry_behind_ms: Option<u64>,
    /// The host's end mark of this wait's entry came.
    pub host_ended_entry: bool,
    pub host_in_scene: bool,
    pub here_in_scene: bool,
    /// The radio line's time grew since the previous poll of this wait.
    pub line_advancing: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OpenWait {
    index: u32,
    since_ms: u64,
}

#[derive(Debug, Clone, Default)]
struct WaitCounter {
    run: Option<RadioWaitRun>,
    begun: u32,
    open: Option<OpenWait>,
}

/// The host's side: numbers the radio waits of each streamed handler and says which one ended.
#[derive(Debug, Clone, Default)]
pub struct HostRadioWaits {
    counter: WaitCounter,
}

/// The guest's side: ends its copy of a radio wait when the host's game says that wait is over.
#[derive(Debug, Clone, Default)]
pub struct GuestRadioWaits {
    counter: WaitCounter,
    marks: Vec<RadioWaitMark>,
}

impl WaitCounter {
    const fn new() -> Self {
        Self {
            run: None,
            begun: 0,
            open: None,
        }
    }

    fn note(&mut self, poll: &RadioWaitPoll) -> OpenWait {
        if self.run != Some(poll.run) {
            self.run = Some(poll.run);
            self.begun = 0;
            self.open = None;
        }
        if let Some(open) = self.open
            && !poll.starts
        {
            return open;
        }
        // A poll with no start seen (the hook came mid-wait) counts as a wait of its own.
        let open = OpenWait {
            index: self.begun,
            since_ms: poll.now_ms,
        };
        self.begun = self.begun.saturating_add(1);
        self.open = Some(open);
        open
    }
}

impl HostRadioWaits {
    pub const fn new() -> Self {
        Self {
            counter: WaitCounter::new(),
        }
    }

    /// Returns the mark of a stream entry's wait that ended in this call; any other wait, or one
    /// still waiting, gives nothing.
    pub fn polled(&mut self, poll: RadioWaitPoll) -> Option<RadioWaitMark> {
        let open = self.counter.note(&poll);
        if poll.still_waiting {
            return None;
        }
        self.counter.open = None;
        let RadioWaitRun::Entry { epoch, seq } = poll.run else {
            return None;
        };
        Some(RadioWaitMark {
            epoch,
            seq,
            wait_index: open.index,
            host_waited_ms: poll.now_ms.saturating_sub(open.since_ms),
        })
    }
}

impl GuestRadioWaits {
    pub const fn new() -> Self {
        Self {
            counter: WaitCounter::new(),
            marks: Vec::new(),
        }
    }

    /// The host's wait `mark` ended. A mark of an entry this game has already passed is dropped.
    pub fn receive_mark(&mut self, mark: RadioWaitMark) {
        if let Some(RadioWaitRun::Entry { epoch, seq }) = self.counter.run
            && (mark.epoch, mark.seq) < (epoch, seq)
        {
            return;
        }
        if self.marks.len() >= MAX_RADIO_WAIT_MARKS_KEPT {
            self.marks.remove(0);
        }
        self.marks.push(mark);
    }

    pub fn forget_marks(&mut self) {
        self.marks.clear();
    }

    pub fn marks_kept(&self) -> usize {
        self.marks.len()
    }

    /// Returns why this game's wait ends now, or nothing while the game's own wait decides.
    pub fn polled(
        &mut self,
        poll: RadioWaitPoll,
        context: RadioWaitContext,
    ) -> Option<RadioWaitRelease> {
        let run_changed = self.counter.run != Some(poll.run);
        let open = self.counter.note(&poll);
        if run_changed && let RadioWaitRun::Entry { epoch, seq } = poll.run {
            self.marks
                .retain(|mark| (mark.epoch, mark.seq) >= (epoch, seq));
        }
        if !poll.still_waiting {
            self.counter.open = None;
            return None;
        }
        let waited_ms = poll.now_ms.saturating_sub(open.since_ms);
        let release = self.release(poll.run, open.index, waited_ms, context);
        if release.is_some() {
            self.counter.open = None;
        }
        release
    }

    fn release(
        &self,
        run: RadioWaitRun,
        index: u32,
        waited_ms: u64,
        context: RadioWaitContext,
    ) -> Option<RadioWaitRelease> {
        if context.host_in_scene && !context.here_in_scene {
            return Some(RadioWaitRelease::HostInScene);
        }
        if let RadioWaitRun::Entry { epoch, seq } = run {
            let mark = self
                .marks
                .iter()
                .find(|mark| mark.epoch == epoch && mark.seq == seq && mark.wait_index == index);
            if let Some(mark) = mark {
                // The host had this wait: this copy lasts as long as the host's did, no less.
                let slack_ms = if context.line_advancing {
                    RADIO_WAIT_MARK_SLACK_FOR_AN_ADVANCING_LINE_MS
                } else {
                    0
                };
                return (waited_ms >= mark.host_waited_ms + slack_ms).then_some(
                    RadioWaitRelease::Mark {
                        line_advancing: context.line_advancing,
                    },
                );
            }
            if context.host_ended_entry {
                return Some(RadioWaitRelease::HostMovedOn);
            }
        }
        context
            .entry_behind_ms
            .is_some_and(|behind_ms| behind_ms > RADIO_WAIT_BEHIND_STREAM_GRACE_MS)
            .then_some(RadioWaitRelease::HostMovedOn)
    }
}

pub fn format_radio_wait_release(handler: &str, release: RadioWaitRelease) -> String {
    let why = match release {
        RadioWaitRelease::Mark {
            line_advancing: false,
        } => "mark, line stalled",
        RadioWaitRelease::Mark {
            line_advancing: true,
        } => "mark, line still playing",
        RadioWaitRelease::HostMovedOn => "the host moved on",
        RadioWaitRelease::HostInScene => "the host is in a scene",
    };
    format!("radio wait: ended your wait in {handler} ({why})")
}
