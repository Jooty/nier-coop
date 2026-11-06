use std::collections::{HashMap, HashSet};

use rc_proto::NpcId;

use crate::npcs::NpcSighting;

/// Gives each NPC of the phase its network id the same way on both machines: NPCs of one (script
/// group, class) are numbered by spawn serial from 0 at phase start. Splitting by class keeps an
/// NPC that exists on one machine only from shifting the numbers of every other model.
#[derive(Debug, Default)]
pub struct NpcRoster {
    phase: Option<u16>,
    next_ordinal: HashMap<(u16, u16), u16>,
    known: HashMap<(u64, i32), NpcId>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct NpcRosterUpdate {
    pub phase_changed: bool,
    pub seen: Vec<(NpcId, NpcSighting)>,
    pub arrived: Vec<NpcId>,
    pub gone: Vec<NpcId>,
}

impl NpcRoster {
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

    pub fn observe(&mut self, phase: u16, sightings: &[NpcSighting]) -> NpcRosterUpdate {
        let mut update = NpcRosterUpdate::default();
        if self.phase != Some(phase) {
            self.reset();
            self.phase = Some(phase);
            update.phase_changed = true;
        }
        let mut arrivals: Vec<&NpcSighting> = sightings
            .iter()
            .filter(|sighting| !self.known.contains_key(&key_of(sighting)))
            .collect();
        arrivals.sort_by_key(|sighting| (sighting.group, sighting.class, sighting.spawn_serial));
        arrivals.dedup_by_key(|sighting| key_of(sighting));
        for sighting in arrivals {
            let next = self
                .next_ordinal
                .entry((sighting.group, sighting.class))
                .or_insert(0);
            let id = NpcId {
                group: sighting.group,
                class: sighting.class,
                ordinal: *next,
            };
            *next = next.saturating_add(1);
            self.known.insert(key_of(sighting), id);
            update.arrived.push(id);
        }
        let mut present = HashSet::with_capacity(sightings.len());
        for sighting in sightings {
            if let Some(id) = self.known.get(&key_of(sighting)) {
                present.insert(key_of(sighting));
                update.seen.push((*id, *sighting));
            }
        }
        self.known.retain(|key, id| {
            let stays = present.contains(key);
            if !stays {
                update.gone.push(*id);
            }
            stays
        });
        update.seen.sort_by_key(|(id, _)| *id);
        update.seen.dedup_by_key(|(id, _)| *id);
        update.arrived.sort();
        update.gone.sort();
        update
    }
}

fn key_of(sighting: &NpcSighting) -> (u64, i32) {
    (sighting.actor, sighting.spawn_serial)
}
