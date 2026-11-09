use rc_proto::{FightStreamEntry, StoryCallArg};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamEntry {
    pub phase: u16,
    pub epoch: u32,
    pub seq: u64,
    pub name: String,
    pub args: Vec<StoryCallArg>,
}

/// The host's side of the stream: every pop inside a fight window gets the next `seq` of the
/// current epoch. A restart signal or a new phase starts a new epoch at `seq` 0.
#[derive(Debug, Clone, Default)]
pub struct StreamRecorder {
    phase: u16,
    epoch: u32,
    next_seq: u64,
}

impl StreamEntry {
    pub fn to_wire(&self) -> FightStreamEntry {
        FightStreamEntry {
            phase: self.phase,
            epoch: self.epoch,
            seq: self.seq,
            name: self.name.clone(),
            args: self.args.clone(),
        }
    }

    pub fn from_wire(entry: FightStreamEntry) -> Self {
        Self {
            phase: entry.phase,
            epoch: entry.epoch,
            seq: entry.seq,
            name: entry.name,
            args: entry.args,
        }
    }
}

impl StreamRecorder {
    pub const fn new(phase: u16) -> Self {
        Self {
            phase,
            epoch: 0,
            next_seq: 0,
        }
    }

    pub fn record(&mut self, name: &str, args: Vec<StoryCallArg>) -> StreamEntry {
        let entry = StreamEntry {
            phase: self.phase,
            epoch: self.epoch,
            seq: self.next_seq,
            name: name.to_owned(),
            args,
        };
        self.next_seq += 1;
        entry
    }

    pub fn restart(&mut self) -> u32 {
        self.epoch = self.epoch.saturating_add(1);
        self.next_seq = 0;
        self.epoch
    }

    pub fn enter_phase(&mut self, phase: u16) -> u32 {
        self.phase = phase;
        self.restart()
    }

    pub const fn phase(&self) -> u16 {
        self.phase
    }

    pub const fn epoch(&self) -> u32 {
        self.epoch
    }

    pub const fn next_seq(&self) -> u64 {
        self.next_seq
    }
}
