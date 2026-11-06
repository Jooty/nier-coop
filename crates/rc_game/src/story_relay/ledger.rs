use crate::story_relay::{
    INJECTED_CALL_MARK_LIFETIME_MS, LEDGER_MAX_ENTRIES_PER_LIST, STORY_CALL_DEDUPE_WINDOW_MS,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CallKey {
    pub function: u64,
    pub args: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StoryRelayLedger {
    ran_here: Vec<(CallKey, u64)>,
    accepted: Vec<(CallKey, u64)>,
    ran_there: Vec<(CallKey, u64)>,
    injected: Vec<(u64, u64)>,
}

impl StoryRelayLedger {
    pub const fn new() -> Self {
        Self {
            ran_here: Vec::new(),
            accepted: Vec::new(),
            ran_there: Vec::new(),
            injected: Vec::new(),
        }
    }

    pub fn note_ran_here(&mut self, key: CallKey, now_ms: u64) {
        push_capped(&mut self.ran_here, (key, now_ms));
    }

    pub fn accept_trigger(&mut self, key: CallKey, now_ms: u64) -> bool {
        self.expire(now_ms);
        let seen = |list: &[(CallKey, u64)]| list.iter().any(|&(held, _)| held == key);
        if seen(&self.ran_here) || seen(&self.accepted) {
            return false;
        }
        push_capped(&mut self.accepted, (key, now_ms));
        true
    }

    pub fn take_accepted(&mut self, key: CallKey, now_ms: u64) -> bool {
        self.expire(now_ms);
        take_key(&mut self.accepted, key)
    }

    pub fn note_ran_there(&mut self, key: CallKey, now_ms: u64) {
        push_capped(&mut self.ran_there, (key, now_ms));
    }

    pub fn take_ran_there(&mut self, key: CallKey, now_ms: u64) -> bool {
        self.expire(now_ms);
        take_key(&mut self.ran_there, key)
    }

    pub fn note_injected(&mut self, slot: u64, now_ms: u64) {
        push_capped(&mut self.injected, (slot, now_ms));
    }

    pub fn take_injected(&mut self, slot: u64, now_ms: u64) -> bool {
        self.expire(now_ms);
        let Some(position) = self.injected.iter().position(|&(held, _)| held == slot) else {
            return false;
        };
        self.injected.remove(position);
        true
    }

    fn expire(&mut self, now_ms: u64) {
        for list in [&mut self.ran_here, &mut self.accepted, &mut self.ran_there] {
            list.retain(|&(_, at)| now_ms.saturating_sub(at) < STORY_CALL_DEDUPE_WINDOW_MS);
        }
        self.injected
            .retain(|&(_, at)| now_ms.saturating_sub(at) < INJECTED_CALL_MARK_LIFETIME_MS);
    }
}

fn push_capped<T>(list: &mut Vec<T>, entry: T) {
    if list.len() >= LEDGER_MAX_ENTRIES_PER_LIST {
        list.remove(0);
    }
    list.push(entry);
}

fn take_key(list: &mut Vec<(CallKey, u64)>, key: CallKey) -> bool {
    let Some(position) = list.iter().position(|&(held, _)| held == key) else {
        return false;
    };
    list.remove(position);
    true
}
