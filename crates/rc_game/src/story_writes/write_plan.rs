use rc_proto::{MAX_STORY_CHANGES_PER_MESSAGE, StoryChange};

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
    previous: Option<StoryArrays>,
    changed_ms: BTreeMap<StoryKey, u64>,
}

impl HostChangeLog {
    pub const fn new() -> Self {
        Self {
            previous: None,
            changed_ms: BTreeMap::new(),
        }
    }

    pub fn observe(&mut self, live: &StoryArrays, now_ms: u64) {
        if let Some(previous) = &self.previous {
            for change in previous.changes_to(live) {
                self.changed_ms.insert(StoryKey::of(change), now_ms);
            }
        }
        self.changed_ms
            .retain(|_, &mut at| now_ms.saturating_sub(at) < HOST_OWN_CHANGE_PRIORITY_MS);
        self.previous = Some(live.clone());
    }

    pub fn note_applied_from_guest(&mut self, change: StoryChange) {
        if let Some(previous) = &mut self.previous {
            previous.apply(change);
        }
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
