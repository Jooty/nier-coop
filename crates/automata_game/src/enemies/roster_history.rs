use std::collections::{BTreeMap, BTreeSet};

use crate::enemies::{EnemyId, EnemySighting, RosterMismatch};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RosterHistory {
    held_here_since_ms: BTreeMap<EnemyId, u64>,
    held_here_before: BTreeSet<EnemyId>,
    reported_by_the_other: BTreeSet<EnemyId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MismatchPlaces {
    pub here: String,
    pub there: String,
}

impl RosterHistory {
    pub fn note_here(&mut self, now_ms: u64, roster: &[EnemySighting]) {
        let ids: BTreeSet<EnemyId> = roster.iter().map(|sighting| sighting.id).collect();
        self.held_here_since_ms.retain(|id, _| ids.contains(id));
        for id in ids {
            self.held_here_since_ms.entry(id).or_insert(now_ms);
            self.held_here_before.insert(id);
        }
    }

    pub fn note_other(&mut self, roster: &[EnemySighting]) {
        self.reported_by_the_other
            .extend(roster.iter().map(|sighting| sighting.id));
    }

    pub fn forget(&mut self) {
        *self = Self::default();
    }

    pub fn detail_of(
        &self,
        now_ms: u64,
        mismatch: &RosterMismatch,
        places: &MismatchPlaces,
    ) -> String {
        let id = mismatch.id();
        let held_here = match self.held_here_since_ms.get(&id) {
            Some(since_ms) => format!(
                "this copy has had it for {} ms",
                now_ms.saturating_sub(*since_ms)
            ),
            None if self.held_here_before.contains(&id) => {
                "this copy had it before and it is gone here".to_owned()
            }
            None => "this copy has never had it".to_owned(),
        };
        let other_reported = match (
            matches!(mismatch, RosterMismatch::OnlyHere(_)),
            self.reported_by_the_other.contains(&id),
        ) {
            (true, true) => "the other copy reported it before and it is gone there",
            (true, false) => "the other copy has never reported it",
            (false, _) => "the other copy reports it now",
        };
        format!(
            " [{held_here}; story places: here {}, other copy {}; {other_reported}]",
            places.here, places.there
        )
    }
}
