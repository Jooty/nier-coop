use std::collections::{BTreeSet, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalOpen {
    SendToPartner,
    AlreadyShown,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TutorialPrompts {
    shown: BTreeSet<u16>,
    waiting: VecDeque<u16>,
}

impl TutorialPrompts {
    pub const fn new() -> Self {
        Self {
            shown: BTreeSet::new(),
            waiting: VecDeque::new(),
        }
    }

    pub fn local_opened(&mut self, id: u16) -> LocalOpen {
        if self.shown.insert(id) {
            LocalOpen::SendToPartner
        } else {
            LocalOpen::AlreadyShown
        }
    }

    pub fn partner_opened(&mut self, id: u16) -> bool {
        if self.shown.contains(&id) || self.waiting.contains(&id) {
            return false;
        }
        self.waiting.push_back(id);
        true
    }

    pub fn next_to_open(&mut self, free: bool) -> Option<u16> {
        while let Some(id) = self.waiting.front().copied() {
            if self.shown.contains(&id) {
                self.waiting.pop_front();
                continue;
            }
            if !free {
                return None;
            }
            self.waiting.pop_front();
            self.shown.insert(id);
            return Some(id);
        }
        None
    }

    pub fn waiting_count(&self) -> usize {
        self.waiting.len()
    }
}
