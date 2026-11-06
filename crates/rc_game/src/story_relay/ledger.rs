use rc_proto::StoryRefusalReason;

use crate::story_relay::{
    HELD_TRIGGER_ANSWER_TIMEOUT_MS, INJECTED_CALL_MARK_LIFETIME_MS, LEDGER_MAX_ENTRIES_PER_LIST,
    RAN_LOCALLY_AFTER_TIMEOUT_RECORD_MS, STORY_CALL_DEDUPE_WINDOW_MS,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CallKey {
    pub function: u64,
    pub args: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InjectedMatch {
    Pointer,
    Key,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefusedAction {
    NotHeld,
    IgnoreLate,
    ForgetOnly,
    RunHere,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct InjectedMark {
    slot: u64,
    key: CallKey,
    at_ms: u64,
    tick: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StoryRelayLedger {
    ran_here: Vec<(CallKey, u64)>,
    accepted: Vec<(CallKey, u64)>,
    ran_there: Vec<(CallKey, u64)>,
    injected: Vec<InjectedMark>,
    held: Vec<(CallKey, u64)>,
    ran_locally: Vec<(CallKey, u64)>,
    tick: u64,
}

impl StoryRelayLedger {
    pub const fn new() -> Self {
        Self {
            ran_here: Vec::new(),
            accepted: Vec::new(),
            ran_there: Vec::new(),
            injected: Vec::new(),
            held: Vec::new(),
            ran_locally: Vec::new(),
            tick: 0,
        }
    }

    pub fn begin_tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    pub fn note_ran_locally(&mut self, key: CallKey, now_ms: u64) {
        push_capped(&mut self.ran_locally, (key, now_ms));
    }

    pub fn take_ran_locally(&mut self, key: CallKey, now_ms: u64) -> bool {
        self.expire(now_ms);
        take_key(&mut self.ran_locally, key)
    }

    pub fn is_idle(&self) -> bool {
        self.ran_here.is_empty()
            && self.accepted.is_empty()
            && self.ran_there.is_empty()
            && self.injected.is_empty()
            && self.held.is_empty()
            && self.ran_locally.is_empty()
    }

    pub fn clear_session(&mut self) {
        *self = Self {
            tick: self.tick,
            ..Self::new()
        };
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

    pub fn note_injected(&mut self, slot: u64, key: CallKey, now_ms: u64) {
        push_capped(
            &mut self.injected,
            InjectedMark {
                slot,
                key,
                at_ms: now_ms,
                tick: self.tick,
            },
        );
    }

    pub fn take_injected(&mut self, slot: u64, key: CallKey, now_ms: u64) -> Option<InjectedMatch> {
        self.expire(now_ms);
        let by_pointer = self.injected.iter().position(|mark| mark.slot == slot);
        let (position, how) = match by_pointer {
            Some(position) => (position, InjectedMatch::Pointer),
            None => {
                let position = self
                    .injected
                    .iter()
                    .position(|mark| mark.key == key && mark.tick == self.tick)?;
                (position, InjectedMatch::Key)
            }
        };
        self.injected.remove(position);
        Some(how)
    }

    pub fn hold_call(&mut self, key: CallKey, now_ms: u64) {
        if !self.held.iter().any(|&(held, _)| held == key) {
            push_capped(&mut self.held, (key, now_ms));
        }
    }

    pub fn resolve_held(&mut self, key: CallKey) -> bool {
        take_key(&mut self.held, key)
    }

    pub fn on_refused(
        &mut self,
        key: CallKey,
        reason: StoryRefusalReason,
        now_ms: u64,
    ) -> RefusedAction {
        if self.take_ran_locally(key, now_ms) {
            return RefusedAction::IgnoreLate;
        }
        if !self.resolve_held(key) {
            return RefusedAction::NotHeld;
        }
        if reason == StoryRefusalReason::Duplicate {
            RefusedAction::ForgetOnly
        } else {
            RefusedAction::RunHere
        }
    }

    pub fn take_timed_out_held(&mut self, now_ms: u64) -> Vec<CallKey> {
        let mut timed_out = Vec::new();
        self.held.retain(|&(key, at)| {
            let late = now_ms.saturating_sub(at) >= HELD_TRIGGER_ANSWER_TIMEOUT_MS;
            if late {
                timed_out.push(key);
            }
            !late
        });
        timed_out
    }

    fn expire(&mut self, now_ms: u64) {
        for list in [&mut self.ran_here, &mut self.accepted, &mut self.ran_there] {
            list.retain(|&(_, at)| now_ms.saturating_sub(at) < STORY_CALL_DEDUPE_WINDOW_MS);
        }
        self.ran_locally
            .retain(|&(_, at)| now_ms.saturating_sub(at) < RAN_LOCALLY_AFTER_TIMEOUT_RECORD_MS);
        self.injected
            .retain(|mark| now_ms.saturating_sub(mark.at_ms) < INJECTED_CALL_MARK_LIFETIME_MS);
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
