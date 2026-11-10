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
    own_arrivals: HashMap<u16, u16>,
    retired_copies: HashMap<u16, u16>,
    known: HashMap<(u64, i32), KnownEnemy>,
}

#[derive(Debug, Clone, Copy)]
struct KnownEnemy {
    id: EnemyId,
    hp: i32,
    own_arrival: Option<u16>,
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
            let arrivals = self.own_arrivals.entry(sighting.group).or_insert(0);
            let own_arrival = *arrivals;
            *arrivals = arrivals.saturating_add(1);
            self.known.insert(
                key_of(sighting),
                KnownEnemy {
                    id,
                    hp: sighting.hp,
                    own_arrival: Some(own_arrival),
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

    pub fn claim(&mut self, actor: u64, spawn_serial: i32, id: EnemyId, hp: i32) {
        self.known.insert(
            (actor, spawn_serial),
            KnownEnemy {
                id,
                hp,
                own_arrival: None,
            },
        );
        let next = self.next_ordinal.entry(id.group).or_insert(0);
        *next = (*next).max(id.ordinal.saturating_add(1));
    }

    /// On the guest: the game's own new enemy `own` takes the id of the host's created copy
    /// `copy`; the copy's entry moves to an id no host enemy has, so it can be removed.
    pub fn take_over_copy(&mut self, own: EnemyId, copy: EnemyId) -> bool {
        let holds = |id: EnemyId| {
            self.known
                .iter()
                .find(|(_, known)| known.id == id)
                .map(|(key, _)| *key)
        };
        let (Some(own_key), Some(copy_key)) = (holds(own), holds(copy)) else {
            return false;
        };
        let retired = self.retired_copies.entry(copy.group).or_insert(0);
        let retired_id = EnemyId {
            group: copy.group,
            ordinal: u16::MAX.saturating_sub(*retired),
        };
        *retired = retired.saturating_add(1);
        if let Some(known) = self.known.get_mut(&copy_key) {
            known.id = retired_id;
        }
        if let Some(known) = self.known.get_mut(&own_key) {
            known.id = copy;
        }
        true
    }

    /// On the guest: a new enemy whose id the host does not have takes the lowest live host id of
    /// its group that no enemy here holds, so a boss started a second time on one game (its
    /// ordinal ran ahead) still pairs with the host's. Returns each (own id, adopted id).
    pub fn adopt_host_ids(
        &mut self,
        spawned: &[EnemyId],
        host_live: &[EnemyId],
    ) -> Vec<(EnemyId, EnemyId)> {
        let mut held: HashSet<EnemyId> = self.known.values().map(|known| known.id).collect();
        let mut adoptions = Vec::new();
        for own in spawned {
            if host_live.contains(own) {
                continue;
            }
            let Some(adopted) = host_live
                .iter()
                .filter(|host| host.group == own.group && !held.contains(host))
                .min()
                .copied()
            else {
                continue;
            };
            let Some(known) = self.known.values_mut().find(|known| known.id == *own) else {
                continue;
            };
            known.id = adopted;
            held.remove(own);
            held.insert(adopted);
            let next = self.next_ordinal.entry(own.group).or_insert(0);
            *next = (*next).max(adopted.ordinal.saturating_add(1));
            adoptions.push((*own, adopted));
        }
        adoptions
    }

    /// How many enemies of its group this game itself had spawned in the phase before `id`'s
    /// enemy (claimed copies do not count); None for a copy made from the host's spawn.
    pub fn own_arrival_of(&self, id: EnemyId) -> Option<u16> {
        self.known
            .values()
            .find(|known| known.id == id)
            .and_then(|known| known.own_arrival)
    }

    pub fn held_ids(&self) -> HashSet<EnemyId> {
        self.known.values().map(|known| known.id).collect()
    }

    /// Renames each (own id, bound id) whose own id an enemy here holds and whose bound id none
    /// does; returns the renames made.
    pub fn bind_ids(&mut self, bindings: &[(EnemyId, EnemyId)]) -> Vec<(EnemyId, EnemyId)> {
        let mut made = Vec::new();
        for (own, bound) in bindings {
            if self.known.values().any(|known| known.id == *bound) {
                continue;
            }
            let Some(known) = self.known.values_mut().find(|known| known.id == *own) else {
                continue;
            };
            known.id = *bound;
            made.push((*own, *bound));
        }
        made
    }
}

impl RosterUpdate {
    pub fn rename(&mut self, adoptions: &[(EnemyId, EnemyId)]) {
        let renamed = |id: &mut EnemyId| {
            if let Some((_, adopted)) = adoptions.iter().find(|(own, _)| own == id) {
                *id = *adopted;
            }
        };
        self.spawned.iter_mut().for_each(renamed);
        self.seen.iter_mut().for_each(|(id, _)| renamed(id));
        self.spawned.sort();
        self.seen.sort_by_key(|(id, _)| *id);
    }
}

fn key_of(sighting: &EnemySighting) -> (u64, i32) {
    (sighting.actor, sighting.spawn_serial)
}
