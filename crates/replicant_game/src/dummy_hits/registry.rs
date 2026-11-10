use crate::dummy_hits::MAX_DUMMY_CALLBACKS;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Entry {
    phase: u16,
    callback: String,
    runs_here: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DummyHitCallbacks {
    entries: Vec<Entry>,
}

impl DummyHitCallbacks {
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn register(&mut self, phase: u16, callback: &str) -> bool {
        if let Some(entry) = self.find_mut(phase, callback) {
            entry.runs_here = 0;
            return true;
        }
        if self.entries.len() >= MAX_DUMMY_CALLBACKS {
            return false;
        }
        self.entries.push(Entry {
            phase,
            callback: callback.to_owned(),
            runs_here: 0,
        });
        true
    }

    pub fn is_registered(&self, phase: u16, callback: &str) -> bool {
        self.entries
            .iter()
            .any(|entry| entry.phase == phase && entry.callback == callback)
    }

    pub fn count_run(&mut self, phase: u16, callback: &str) -> u32 {
        self.find_mut(phase, callback).map_or(0, |entry| {
            entry.runs_here = entry.runs_here.saturating_add(1);
            entry.runs_here
        })
    }

    fn find_mut(&mut self, phase: u16, callback: &str) -> Option<&mut Entry> {
        self.entries
            .iter_mut()
            .find(|entry| entry.phase == phase && entry.callback == callback)
    }
}

pub fn dummy_hit_log_line(callback: &str, runs_here: u32) -> String {
    format!("dummy hit: {callback} ran here, runs here {runs_here}")
}
