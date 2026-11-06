use std::collections::{BTreeMap, BTreeSet};

use rc_proto::{StoryArray, StoryArrayPart, StoryChange};

use crate::story_progress::{
    STORY_DRIFT_GRACE_MS, STORY_DRIFT_KEYS_THAT_MEAN_A_RELOAD, StoryArrays, StoryKey,
};

/// The guest's copy of the host's story state. A difference the host caused is fixed at once; one
/// the guest's own scripts made waits [`STORY_DRIFT_GRACE_MS`], as the host usually makes it too.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StoryMirror {
    host: StoryArrays,
    received: BTreeSet<StoryArray>,
    urgent_arrays: BTreeSet<StoryArray>,
    urgent_keys: BTreeSet<StoryKey>,
    drift_since: BTreeMap<StoryKey, u64>,
    held_until: BTreeMap<StoryKey, u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StoryPlan {
    pub fixes: Vec<StoryChange>,
    pub held: usize,
    pub reload_restored: bool,
}

impl StoryMirror {
    pub fn is_complete(&self) -> bool {
        self.received.len() == StoryArray::ALL.len()
    }

    pub fn receive_part(&mut self, part: &StoryArrayPart) -> bool {
        if !self.host.set_bytes(part.array, &part.bytes) {
            return false;
        }
        self.received.insert(part.array);
        self.urgent_arrays.insert(part.array);
        true
    }

    pub fn receive_changes(&mut self, changes: &[StoryChange]) -> bool {
        if !self.is_complete() {
            return false;
        }
        for &change in changes {
            self.host.apply(change);
            self.urgent_keys.insert(StoryKey::of(change));
        }
        true
    }

    pub fn hold(&mut self, key: StoryKey, until_ms: u64) {
        self.held_until.insert(key, until_ms);
    }

    pub fn host_digest(&self) -> Option<u64> {
        self.is_complete().then(|| self.host.digest())
    }

    pub fn host(&self) -> Option<&StoryArrays> {
        self.is_complete().then_some(&self.host)
    }

    pub fn plan(&mut self, live: &StoryArrays, now_ms: u64) -> StoryPlan {
        let mut plan = StoryPlan::default();
        if !self.is_complete() {
            return plan;
        }
        self.held_until.retain(|_, &mut until| until > now_ms);
        let differences = live.changes_to(&self.host);
        let differing: BTreeSet<StoryKey> = differences.iter().copied().map(StoryKey::of).collect();
        self.drift_since.retain(|key, _| differing.contains(key));
        self.urgent_keys.retain(|key| differing.contains(key));
        let own_drift = differences
            .iter()
            .filter(|change| !self.is_urgent(StoryKey::of(**change)))
            .count();
        plan.reload_restored = own_drift > STORY_DRIFT_KEYS_THAT_MEAN_A_RELOAD;
        for change in differences {
            let key = StoryKey::of(change);
            let since = *self.drift_since.entry(key).or_insert(now_ms);
            let held_by_write = self.held_until.contains_key(&key) && !self.is_urgent(key);
            let due = !held_by_write
                && (plan.reload_restored
                    || self.is_urgent(key)
                    || now_ms.saturating_sub(since) >= STORY_DRIFT_GRACE_MS);
            if due {
                plan.fixes.push(change);
                self.drift_since.insert(key, now_ms);
            } else {
                plan.held += 1;
            }
        }
        self.urgent_arrays.clear();
        self.urgent_keys.clear();
        plan
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    fn is_urgent(&self, key: StoryKey) -> bool {
        self.urgent_arrays.contains(&key.array) || self.urgent_keys.contains(&key)
    }
}
