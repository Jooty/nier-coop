use crate::boss_messages::RAISED_SEQ_MEMORY_ENTRIES;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RaisedSeqs {
    recent: Vec<u32>,
}

impl RaisedSeqs {
    pub const fn new() -> Self {
        Self { recent: Vec::new() }
    }

    pub fn allow(&mut self, seq: u32) -> bool {
        if self.recent.contains(&seq) {
            return false;
        }
        if self.recent.len() >= RAISED_SEQ_MEMORY_ENTRIES {
            self.recent.remove(0);
        }
        self.recent.push(seq);
        true
    }

    pub fn clear(&mut self) {
        self.recent.clear();
    }
}
