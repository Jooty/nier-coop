use std::collections::{BTreeMap, BTreeSet};

use crate::enemies::{
    CENSUS_LINES_LOGGED_PER_MINUTE, EnemyId, EnemySighting,
    MOST_ENEMIES_NAMED_ONE_BY_ONE_IN_ONE_CENSUS, ROSTER_MISMATCH_MUST_LAST_MS,
};
use crate::minute_line_budget::MinuteLineBudget;

const CENSUS_LINES_TOPIC: &str = "enemies: seen and gone lines";

/// What one copy knew of its placed enemies at the last count, and the log lines a new count gives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Census {
    known: BTreeMap<EnemyId, EnemySighting>,
    doubled: BTreeSet<EnemyId>,
    budget: MinuteLineBudget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RosterMismatch {
    OnlyHere(EnemySighting),
    OnlyThere(EnemySighting),
    OtherObject {
        here: EnemySighting,
        there: EnemySighting,
    },
}

/// Says a mismatch once it has lasted, and says once more when it is over.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MismatchWatch {
    open: BTreeMap<(EnemyId, MismatchKind), OpenMismatch>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MismatchKind {
    OnlyHere,
    OnlyThere,
    OtherObject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OpenMismatch {
    since_ms: u64,
    said: bool,
}

impl Default for Census {
    fn default() -> Self {
        Self {
            known: BTreeMap::new(),
            doubled: BTreeSet::new(),
            budget: MinuteLineBudget::new(CENSUS_LINES_LOGGED_PER_MINUTE),
        }
    }
}

impl Census {
    /// Takes this count's enemies (one per id) and the ids left out because two enemies had them.
    pub fn note(
        &mut self,
        now_ms: u64,
        sightings: &[EnemySighting],
        doubled: &[EnemyId],
    ) -> Vec<String> {
        let now: BTreeMap<EnemyId, EnemySighting> = sightings
            .iter()
            .map(|sighting| (sighting.id, *sighting))
            .collect();
        let seen: Vec<String> = now
            .values()
            .filter(|sighting| !self.known.contains_key(&sighting.id))
            .map(EnemySighting::label)
            .collect();
        let gone: Vec<String> = self
            .known
            .values()
            .filter(|sighting| !now.contains_key(&sighting.id))
            .map(EnemySighting::label)
            .collect();
        let now_doubled: BTreeSet<EnemyId> = doubled.iter().copied().collect();
        let mut lines = Vec::new();
        if let Some(line) = named_line("seen", &seen) {
            lines.push(line);
        }
        if let Some(line) = named_line("gone", &gone) {
            lines.push(line);
        }
        if now_doubled != self.doubled && !now_doubled.is_empty() {
            let labels: Vec<String> = now_doubled.iter().map(|id| id.label()).collect();
            lines.extend(named_line(
                "two live enemies share one id and are left out:",
                &labels,
            ));
        }
        self.known = now;
        self.doubled = now_doubled;
        lines
            .into_iter()
            .flat_map(|line| self.budget.lines_for(now_ms, CENSUS_LINES_TOPIC, line))
            .collect()
    }

    pub fn known(&self) -> Vec<EnemySighting> {
        self.known.values().copied().collect()
    }
}

impl RosterMismatch {
    pub fn id(&self) -> EnemyId {
        match self {
            Self::OnlyHere(sighting) | Self::OnlyThere(sighting) => sighting.id,
            Self::OtherObject { here, .. } => here.id,
        }
    }

    pub fn line(&self) -> String {
        match self {
            Self::OnlyHere(sighting) => {
                format!("enemies: mismatch: only this copy has {}", sighting.label())
            }
            Self::OnlyThere(sighting) => format!(
                "enemies: mismatch: only the other copy has {}",
                sighting.label()
            ),
            Self::OtherObject { here, there } => format!(
                "enemies: mismatch: {} is object {:#X} here and object {:#X} on the other copy",
                here.id.label(),
                here.object_id,
                there.object_id
            ),
        }
    }

    fn kind(&self) -> MismatchKind {
        match self {
            Self::OnlyHere(_) => MismatchKind::OnlyHere,
            Self::OnlyThere(_) => MismatchKind::OnlyThere,
            Self::OtherObject { .. } => MismatchKind::OtherObject,
        }
    }
}

impl MismatchWatch {
    pub fn note(&mut self, now_ms: u64, mismatches: &[RosterMismatch]) -> Vec<String> {
        self.note_detailed(now_ms, mismatches, &|_| String::new())
    }

    pub fn note_detailed(
        &mut self,
        now_ms: u64,
        mismatches: &[RosterMismatch],
        detail: &dyn Fn(&RosterMismatch) -> String,
    ) -> Vec<String> {
        let mut lines = Vec::new();
        let current: BTreeSet<(EnemyId, MismatchKind)> = mismatches
            .iter()
            .map(|mismatch| (mismatch.id(), mismatch.kind()))
            .collect();
        self.open.retain(|(id, kind), open| {
            let stays = current.contains(&(*id, *kind));
            if !stays && open.said {
                lines.push(format!("enemies: the mismatch over {} is over", id.label()));
            }
            stays
        });
        for mismatch in mismatches {
            let open = self
                .open
                .entry((mismatch.id(), mismatch.kind()))
                .or_insert(OpenMismatch {
                    since_ms: now_ms,
                    said: false,
                });
            if !open.said && now_ms.saturating_sub(open.since_ms) >= ROSTER_MISMATCH_MUST_LAST_MS {
                open.said = true;
                lines.push(format!("{}{}", mismatch.line(), detail(mismatch)));
            }
        }
        lines
    }

    pub fn forget(&mut self) {
        self.open.clear();
    }
}

/// One sighting per id, in id order, and the ids that more than one live enemy carried.
pub fn distinct_sightings(all: &[EnemySighting]) -> (Vec<EnemySighting>, Vec<EnemyId>) {
    let mut by_id: BTreeMap<EnemyId, Vec<EnemySighting>> = BTreeMap::new();
    for sighting in all {
        by_id.entry(sighting.id).or_default().push(*sighting);
    }
    let doubled = by_id
        .iter()
        .filter(|(_, found)| found.len() > 1)
        .map(|(id, _)| *id)
        .collect();
    let distinct = by_id
        .into_values()
        .filter(|found| found.len() == 1)
        .map(|found| found[0])
        .collect();
    (distinct, doubled)
}

pub fn roster_mismatches(here: &[EnemySighting], there: &[EnemySighting]) -> Vec<RosterMismatch> {
    let theirs: BTreeMap<EnemyId, EnemySighting> = there
        .iter()
        .map(|sighting| (sighting.id, *sighting))
        .collect();
    let ours: BTreeSet<EnemyId> = here.iter().map(|sighting| sighting.id).collect();
    let mut mismatches: Vec<RosterMismatch> = here
        .iter()
        .filter_map(|mine| match theirs.get(&mine.id) {
            None => Some(RosterMismatch::OnlyHere(*mine)),
            Some(other) if other.object_id != mine.object_id => Some(RosterMismatch::OtherObject {
                here: *mine,
                there: *other,
            }),
            Some(_) => None,
        })
        .collect();
    mismatches.extend(
        there
            .iter()
            .filter(|other| !ours.contains(&other.id))
            .map(|other| RosterMismatch::OnlyThere(*other)),
    );
    mismatches
}

fn named_line(what: &str, labels: &[String]) -> Option<String> {
    if labels.is_empty() {
        return None;
    }
    let named = labels
        .iter()
        .take(MOST_ENEMIES_NAMED_ONE_BY_ONE_IN_ONE_CENSUS)
        .cloned()
        .collect::<Vec<_>>()
        .join("; ");
    let more = labels
        .len()
        .saturating_sub(MOST_ENEMIES_NAMED_ONE_BY_ONE_IN_ONE_CENSUS);
    Some(if more == 0 {
        format!("enemies: {what} {named}")
    } else {
        format!("enemies: {what} {named}; and {more} more")
    })
}
