const MAX_UNUSED_ANNOUNCEMENTS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Announcement {
    name: String,
    announced_ms: u64,
}

#[derive(Debug, Clone, Default)]
pub struct HostLedRuns {
    phase: Option<u16>,
    unused: Vec<Announcement>,
    running: Option<Announcement>,
}

impl HostLedRuns {
    pub const fn new() -> Self {
        Self {
            phase: None,
            unused: Vec::new(),
            running: None,
        }
    }

    pub fn announce(&mut self, name: &str, now_ms: u64) {
        if self.unused.len() >= MAX_UNUSED_ANNOUNCEMENTS {
            self.unused.remove(0);
        }
        self.unused.push(Announcement {
            name: name.to_owned(),
            announced_ms: now_ms,
        });
    }

    /// A handler was popped into the script slot; the first announcement of its name is used up.
    pub fn holder_popped(&mut self, name: &str) {
        self.running = self
            .unused
            .iter()
            .position(|announcement| announcement.name == name)
            .map(|index| self.unused.remove(index));
    }

    /// The script slot went idle, so the run in it is over.
    pub fn slot_idle(&mut self) {
        self.running = None;
    }

    /// Unused announcements are dropped when the phase changes; a run in the slot goes on.
    pub fn enter_phase(&mut self, phase: Option<u16>) {
        if phase != self.phase {
            self.phase = phase;
            self.unused.clear();
        }
    }

    /// When the host's game announced the handler now in the slot, if it is a host-led run.
    pub fn running_since_ms(&self) -> Option<u64> {
        self.running
            .as_ref()
            .map(|announcement| announcement.announced_ms)
    }

    pub fn running_name(&self) -> Option<&str> {
        self.running
            .as_ref()
            .map(|announcement| announcement.name.as_str())
    }
}
