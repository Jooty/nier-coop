/// What the stream's copy of the start handler does when it pops on the guest's game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartStreamPop {
    Run,
    /// Another copy ran here while this one sat in the engine queue: it is not run again.
    AlreadyRan,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum StartEntryState {
    #[default]
    None,
    /// Not run here yet: the stream runs it as entry 0 unless another copy runs first.
    Waiting(String),
    /// Another copy ran here first: the stream's copy is skipped when it arrives or pops.
    RanHereFirst(String),
    /// The stream's copy ran: any other copy that pops in this window is not run.
    RanFromStream(String),
}

/// Entry 0 of a host window that a start id's pop armed: the fight's start handler. On the
/// guest it runs exactly once, from the stream or from another copy, whichever runs first.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StartEntry {
    state: StartEntryState,
}

impl StartEntry {
    pub const fn new() -> Self {
        Self {
            state: StartEntryState::None,
        }
    }

    pub fn forget(&mut self) {
        self.state = StartEntryState::None;
    }

    /// The host's window opened with `name` as its entry 0.
    pub fn expect(&mut self, name: &str, ran_here_already: bool) {
        self.state = if ran_here_already {
            StartEntryState::RanHereFirst(name.to_owned())
        } else {
            StartEntryState::Waiting(name.to_owned())
        };
    }

    /// `name` is this window's start entry and the stream's copy of it has not popped yet.
    pub fn is_expected(&self, name: &str) -> bool {
        self.is_waiting_for(name) || self.skips_stream_copy_of(name)
    }

    pub fn is_waiting_for(&self, name: &str) -> bool {
        matches!(&self.state, StartEntryState::Waiting(waiting) if waiting == name)
    }

    pub fn skips_stream_copy_of(&self, name: &str) -> bool {
        matches!(&self.state, StartEntryState::RanHereFirst(ran) if ran == name)
    }

    pub fn stream_copy_skipped(&mut self) {
        if matches!(self.state, StartEntryState::RanHereFirst(_)) {
            self.state = StartEntryState::None;
        }
    }

    /// A copy of `name` that is not the stream's ran on this game: true when the stream's copy
    /// was still waiting, which must now not run.
    pub fn another_copy_ran(&mut self, name: &str) -> bool {
        if !self.is_waiting_for(name) {
            return false;
        }
        self.state = StartEntryState::RanHereFirst(name.to_owned());
        true
    }

    /// The stream's entry 0 `name` popped from the engine queue; `None` when it is no start entry.
    pub fn stream_copy_popped(&mut self, name: &str) -> Option<StartStreamPop> {
        if self.is_waiting_for(name) {
            self.state = StartEntryState::RanFromStream(name.to_owned());
            return Some(StartStreamPop::Run);
        }
        if self.skips_stream_copy_of(name) {
            self.state = StartEntryState::None;
            return Some(StartStreamPop::AlreadyRan);
        }
        None
    }

    pub fn holds_other_copies_of(&self, name: &str) -> bool {
        matches!(&self.state, StartEntryState::RanFromStream(ran) if ran == name)
    }

    /// The stream's copy may be queued now. It never starts in the middle of this game's own
    /// event scene: the scene's own end runs first.
    pub fn may_release(&self, name: &str, own_scene_playing: bool) -> bool {
        !(self.is_waiting_for(name) && own_scene_playing)
    }
}
