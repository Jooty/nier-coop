use std::collections::{HashMap, HashSet};

use rc_proto::{EnemyGoneReason, EnemyId};

use crate::enemies::EnemySighting;

/// Gives each enemy of the current phase its network id, (script group, spawn ordinal), the same
/// way on both machines: new enemies of a group are numbered in the order this machine spawned
/// them (its spawn serial), counting from 0 when the phase begins.
#[derive(Debug, Default)]
pub struct EnemyRoster {
    phase: Option<u16>,
    next_ordinal: HashMap<u16, u16>,
    known: HashMap<(u64, i32), KnownEnemy>,
}

#[derive(Debug, Clone, Copy)]
struct KnownEnemy {
    id: EnemyId,
    hp: i32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RosterUpdate {
    pub phase_changed: bool,
    pub unloaded: Vec<EnemyId>,
    pub seen: Vec<(EnemyId, EnemySighting)>,
    pub spawned: Vec<EnemyId>,
    pub gone: Vec<(EnemyId, EnemyGoneReason)>,
}

impl EnemyRoster {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn phase(&self) -> Option<u16> {
        self.phase
    }

    pub fn len(&self) -> usize {
        self.known.len()
    }

    pub fn is_empty(&self) -> bool {
        self.known.is_empty()
    }

    pub fn observe(&mut self, phase: u16, sightings: &[EnemySighting]) -> RosterUpdate {
        let mut update = RosterUpdate::default();
        if self.phase != Some(phase) {
            update.unloaded = self.known.values().map(|known| known.id).collect();
            update.unloaded.sort();
            self.reset();
            self.phase = Some(phase);
            update.phase_changed = true;
        }
        let mut arrivals: Vec<&EnemySighting> = sightings
            .iter()
            .filter(|sighting| !self.known.contains_key(&key_of(sighting)))
            .collect();
        arrivals.sort_by_key(|sighting| (sighting.group, sighting.spawn_serial));
        arrivals.dedup_by_key(|sighting| key_of(sighting));
        for sighting in arrivals {
            let next = self.next_ordinal.entry(sighting.group).or_insert(0);
            let id = EnemyId {
                group: sighting.group,
                ordinal: *next,
            };
            *next = next.saturating_add(1);
            self.known.insert(
                key_of(sighting),
                KnownEnemy {
                    id,
                    hp: sighting.hp,
                },
            );
            update.spawned.push(id);
        }
        let mut present = HashSet::with_capacity(sightings.len());
        for sighting in sightings {
            if let Some(known) = self.known.get_mut(&key_of(sighting)) {
                known.hp = sighting.hp;
                present.insert(key_of(sighting));
                update.seen.push((known.id, *sighting));
            }
        }
        self.known.retain(|key, known| {
            let stays = present.contains(key);
            if !stays {
                let reason = if known.hp <= 0 {
                    EnemyGoneReason::Killed
                } else {
                    EnemyGoneReason::Removed
                };
                update.gone.push((known.id, reason));
            }
            stays
        });
        update.seen.sort_by_key(|(id, _)| *id);
        update.seen.dedup_by_key(|(id, _)| *id);
        update.spawned.sort();
        update.gone.sort_by_key(|(id, _)| *id);
        update
    }
}

fn key_of(sighting: &EnemySighting) -> (u64, i32) {
    (sighting.actor, sighting.spawn_serial)
}
