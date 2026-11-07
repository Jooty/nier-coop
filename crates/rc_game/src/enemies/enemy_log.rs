use std::collections::{BTreeMap, BTreeSet};

use rc_proto::{EnemyGoneReason, EnemyId, EnemyState};

use crate::enemies::{EnemySighting, RemovalCause, RosterUpdate};

#[derive(Debug, Clone, Copy)]
struct LastSeen {
    class_id: i32,
    position: [f32; 3],
    hp: i32,
    max_hp: i32,
}

#[derive(Debug, Default)]
pub struct EnemyLog {
    last: BTreeMap<EnemyId, LastSeen>,
    causes: BTreeMap<EnemyId, RemovalCause>,
    group_sizes: BTreeMap<u16, usize>,
}

impl EnemyLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn note_cause(&mut self, id: EnemyId, cause: RemovalCause) {
        self.causes.insert(id, cause);
    }

    pub fn observe(
        &mut self,
        update: &RosterUpdate,
        spawn_cause: impl Fn(EnemyId) -> String,
    ) -> Vec<String> {
        let mut lines = Vec::new();
        for id in &update.unloaded {
            if let Some(last) = self.last.get(id) {
                lines.push(removal_line(*id, last, RemovalCause::ZoneUnload));
            }
        }
        if !update.unloaded.is_empty() || update.phase_changed {
            self.last.clear();
            self.causes.clear();
            self.group_sizes.clear();
        }
        for (id, sighting) in &update.seen {
            let known = self.last.insert(*id, last_seen_of(sighting));
            if known.is_none() && update.spawned.contains(id) {
                lines.push(spawn_line(*id, sighting, &spawn_cause(*id)));
            }
        }
        for (id, reason) in &update.gone {
            let cause = self.causes.remove(id).unwrap_or(match reason {
                EnemyGoneReason::Killed => RemovalCause::KilledLocally,
                EnemyGoneReason::Removed => RemovalCause::GameRemoval,
            });
            if let Some(last) = self.last.remove(id) {
                lines.push(removal_line(*id, &last, cause));
            }
        }
        lines.extend(self.group_lines(update));
        lines
    }

    fn group_lines(&mut self, update: &RosterUpdate) -> Vec<String> {
        let mut sizes: BTreeMap<u16, usize> = BTreeMap::new();
        for (id, _) in &update.seen {
            *sizes.entry(id.group).or_insert(0) += 1;
        }
        let mut lines = Vec::new();
        for (group, count) in &sizes {
            if !self.group_sizes.contains_key(group) {
                lines.push(format!("group {group} registered with {count} enemies"));
            }
        }
        let emptied: BTreeSet<u16> = self
            .group_sizes
            .keys()
            .filter(|group| !sizes.contains_key(group))
            .copied()
            .collect();
        for group in emptied {
            lines.push(format!("group {group} is empty"));
        }
        self.group_sizes = sizes;
        lines
    }
}

pub fn relayed_spawn_text(enemy: &EnemyState) -> String {
    format!(
        "spawn {} group {} hp {}/{} at {}, cause: relayed from the host",
        id_text(enemy.id),
        enemy.id.group,
        enemy.hp,
        enemy.max_hp,
        position_text(enemy.position)
    )
}

pub fn sighting_text(id: EnemyId, sighting: &EnemySighting) -> String {
    format!(
        "{} group {} class {:#x} hp {}/{} at {}",
        id_text(id),
        id.group,
        sighting.class_id,
        sighting.hp,
        sighting.max_hp,
        position_text(sighting.position)
    )
}

fn last_seen_of(sighting: &EnemySighting) -> LastSeen {
    LastSeen {
        class_id: sighting.class_id,
        position: sighting.position,
        hp: sighting.hp,
        max_hp: sighting.max_hp,
    }
}

fn spawn_line(id: EnemyId, sighting: &EnemySighting, cause: &str) -> String {
    format!("spawn {}, cause: {cause}", sighting_text(id, sighting))
}

fn removal_line(id: EnemyId, last: &LastSeen, cause: RemovalCause) -> String {
    format!(
        "removed {} group {} class {:#x} hp {}/{} last at {}, cause: {}",
        id_text(id),
        id.group,
        last.class_id,
        last.hp,
        last.max_hp,
        position_text(last.position),
        cause.text()
    )
}

fn id_text(id: EnemyId) -> String {
    format!("{}:{}", id.group, id.ordinal)
}

fn position_text(position: [f32; 3]) -> String {
    format!(
        "({:.1}, {:.1}, {:.1})",
        position[0], position[1], position[2]
    )
}
