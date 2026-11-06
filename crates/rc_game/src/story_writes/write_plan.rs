use rc_proto::{MAX_STORY_CHANGES_PER_MESSAGE, StoryChange};

use crate::story_progress::StoryArrays;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HostWriteOutcome {
    pub apply: Vec<StoryChange>,
    pub already_held: usize,
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
pub fn host_write_outcome(live: &StoryArrays, changes: &[StoryChange]) -> HostWriteOutcome {
    let mut outcome = HostWriteOutcome::default();
    for &change in changes {
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
