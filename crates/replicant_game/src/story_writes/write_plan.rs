use replicant_proto::{MAX_STORY_CHANGES_PER_MESSAGE, StoryChange};

use std::collections::BTreeMap;

use crate::story_progress::{StoryArrays, StoryKey};
use crate::story_writes::HOST_OWN_CHANGE_PRIORITY_MS;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HostWriteOutcome {
    pub apply: Vec<StoryChange>,
    pub already_held: usize,
    pub ignored_recent_host_change: usize,
}

/// Which story ids the host's own game changed lately, so a guest's talk write cannot revert them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HostChangeLog {
    applied_from_guest: Vec<(StoryChange, u64)>,
    changed_ms: BTreeMap<StoryKey, u64>,
}

impl HostChangeLog {
    pub const fn new() -> Self {
        Self {
            applied_from_guest: Vec::new(),
            changed_ms: BTreeMap::new(),
        }
    }

    pub fn note_feed_changes(&mut self, changes: &[StoryChange], now_ms: u64) {
        self.applied_from_guest
            .retain(|&(_, at)| now_ms.saturating_sub(at) < HOST_OWN_CHANGE_PRIORITY_MS);
        for &change in changes {
            let ours = self
                .applied_from_guest
                .iter()
                .position(|&(applied, _)| applied == change);
            match ours {
                Some(position) => {
                    self.applied_from_guest.remove(position);
                }
                None => {
                    self.changed_ms.insert(StoryKey::of(change), now_ms);
                }
            }
        }
        self.changed_ms
            .retain(|_, &mut at| now_ms.saturating_sub(at) < HOST_OWN_CHANGE_PRIORITY_MS);
    }

    pub fn note_applied_from_guest(&mut self, change: StoryChange, now_ms: u64) {
        self.applied_from_guest.push((change, now_ms));
    }

    pub fn changed_recently(&self, key: StoryKey, now_ms: u64) -> bool {
        self.changed_ms
            .get(&key)
            .is_some_and(|&at| now_ms.saturating_sub(at) < HOST_OWN_CHANGE_PRIORITY_MS)
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

/// What the talker's callback changed, in message-sized batches (clears first, then sets, counters last).
pub fn talker_write_batches(before: &StoryArrays, after: &StoryArrays) -> Vec<Vec<StoryChange>> {
    before
        .changes_to(after)
        .chunks(MAX_STORY_CHANGES_PER_MESSAGE)
        .map(<[StoryChange]>::to_vec)
        .collect()
}

/// The part of a guest's write the host's world does not already hold.
pub fn host_write_outcome(
    live: &StoryArrays,
    changes: &[StoryChange],
    host_changes: &HostChangeLog,
    now_ms: u64,
) -> HostWriteOutcome {
    let mut outcome = HostWriteOutcome::default();
    for &change in changes {
        if host_changes.changed_recently(StoryKey::of(change), now_ms) {
            outcome.ignored_recent_host_change += 1;
            continue;
        }
        let held = match change {
            StoryChange::Flag { array, id, on } => live.flag(array, id) == on,
            StoryChange::Counter { array, id, value } => live.counter(array, id) == value,
        };
        if held {
            outcome.already_held += 1;
        } else {
            outcome.apply.push(change);
        }
    }
    outcome
}

/// Moves a talk-write baseline forward over changes the host's mirror applied, so they are never forwarded back.
pub fn rebaseline_over_mirror_fixes(baseline: &mut StoryArrays, applied: &[StoryChange]) {
    for &change in applied {
        baseline.apply(change);
    }
}
