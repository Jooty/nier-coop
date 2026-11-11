use std::collections::{BTreeMap, BTreeSet};

use crate::enemies::{
    EnemyId, EnemySpawn, GENERATOR_SLOT_STATE_ENEMY_NOT_MADE, GENERATOR_SLOT_STATE_FREED,
    GeneratorSlot, GeneratorState, MOST_HOST_SPAWNS_HELD,
};

/// Why the guest takes a slot out of its own generator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotRemoval {
    NotTheHosts,
    OverOnTheHostBeforeItWasMade,
}

/// What the guest does to one of its generators before the game updates it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GuestGeneratorStep {
    pub adds: Vec<EnemySpawn>,
    pub removals: Vec<(usize, SlotRemoval)>,
    pub finishes: bool,
    pub spawned: Option<u32>,
}

/// The guest's book of the host's spawns and generator states.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GuestGenerators {
    spawns: BTreeMap<EnemyId, HeldSpawn>,
    states: BTreeMap<u32, GeneratorState>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct HeldSpawn {
    spawn: EnemySpawn,
    stage: SpawnStage,
    over: Option<OverOnTheHost>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpawnStage {
    Heard,
    InASlot,
    Made { handle: u32 },
    NotMade,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverOnTheHost {
    Defeated,
    Gone,
}

impl GuestGenerators {
    pub fn hear_spawn(&mut self, spawn: EnemySpawn) -> Vec<String> {
        let mut lines = Vec::new();
        if self.spawns.len() >= MOST_HOST_SPAWNS_HELD
            && let Some((id, _)) = self.spawns.pop_first()
        {
            lines.push(format!(
                "enemies: too many of the host's spawns are held, {} is forgotten",
                id.label()
            ));
        }
        lines.push(format!(
            "enemies: the host's generator made {}",
            spawn.label()
        ));
        self.spawns.insert(
            spawn.id(),
            HeldSpawn {
                spawn,
                stage: SpawnStage::Heard,
                over: None,
            },
        );
        lines
    }

    /// The host's enemy is no longer there: defeated, or taken away by the host's game.
    pub fn hear_gone(&mut self, id: EnemyId, defeated: bool) -> Option<String> {
        let held = self.spawns.get_mut(&id)?;
        held.over = Some(if defeated {
            OverOnTheHost::Defeated
        } else {
            OverOnTheHost::Gone
        });
        if matches!(held.stage, SpawnStage::Heard | SpawnStage::NotMade) {
            self.spawns.remove(&id);
            return Some(format!(
                "enemies: {} is over on the host and was never made here",
                id.label()
            ));
        }
        None
    }

    pub fn hear_states(&mut self, states: &[GeneratorState]) {
        for state in states {
            self.states.insert(state.generator, *state);
        }
    }

    pub fn forget(&mut self) {
        *self = Self::default();
    }

    pub fn spawns_held(&self) -> usize {
        self.spawns.len()
    }

    pub fn host_state(&self, generator: u32) -> Option<GeneratorState> {
        self.states.get(&generator).copied()
    }

    /// Slots no spawn of the host stands behind go, spawns heard and not yet in a slot come, and
    /// the run ends here when it has ended on the host.
    pub fn step(&self, generator: u32, slots: &[GeneratorSlot]) -> GuestGeneratorStep {
        let (owners, _) = self.owners_of(generator, slots);
        let removals = slots
            .iter()
            .zip(&owners)
            .enumerate()
            .filter(|(_, (slot, _))| slot.state != GENERATOR_SLOT_STATE_FREED)
            .filter_map(|(index, (_, owner))| match owner {
                None => Some((index, SlotRemoval::NotTheHosts)),
                Some(id) => {
                    let held = self.spawns.get(id)?;
                    (held.stage == SpawnStage::InASlot && held.over.is_some())
                        .then_some((index, SlotRemoval::OverOnTheHostBeforeItWasMade))
                }
            })
            .collect();
        let adds: Vec<EnemySpawn> = self
            .of_generator(generator)
            .filter(|held| held.stage == SpawnStage::Heard && held.over.is_none())
            .map(|held| held.spawn)
            .collect();
        let state = self.states.get(&generator);
        GuestGeneratorStep {
            finishes: adds.is_empty() && state.is_some_and(|state| state.phase.is_after_a_run()),
            spawned: state.map(|state| state.spawned),
            adds,
            removals,
        }
    }

    pub fn note_added(&mut self, id: EnemyId) {
        if let Some(held) = self.spawns.get_mut(&id) {
            held.stage = SpawnStage::InASlot;
        }
    }

    /// This copy cannot make the spawn: it is not tried again.
    pub fn note_refused(&mut self, id: EnemyId) {
        if let Some(held) = self.spawns.get_mut(&id) {
            held.stage = SpawnStage::NotMade;
        }
    }

    /// The host's spawn a slot that still waits for its enemy was made for.
    pub fn spawn_in_slot(&self, generator: u32, slot: &GeneratorSlot) -> Option<EnemySpawn> {
        self.of_generator(generator)
            .find(|held| held.stage == SpawnStage::InASlot && slot_is_of(slot, &held.spawn))
            .map(|held| held.spawn)
    }

    /// Takes the generator's slots as the game left them after its update.
    pub fn note_slots(&mut self, generator: u32, slots: &[GeneratorSlot]) -> Vec<String> {
        let (owners, owned) = self.owners_of(generator, slots);
        let mut lines = Vec::new();
        for (slot, owner) in slots.iter().zip(&owners) {
            let Some(held) = owner.and_then(|id| self.spawns.get_mut(&id)) else {
                continue;
            };
            if held.stage != SpawnStage::InASlot {
                continue;
            }
            if slot.holds_a_made_enemy() {
                held.stage = SpawnStage::Made {
                    handle: slot.handle,
                };
                lines.push(format!(
                    "enemies: made {} from the host's message with this copy's own generator (handle {:#X})",
                    held.spawn.label(),
                    slot.handle
                ));
            } else if slot.state == GENERATOR_SLOT_STATE_ENEMY_NOT_MADE {
                held.stage = SpawnStage::NotMade;
                lines.push(format!(
                    "enemies: mismatch: this copy's game could not make {}",
                    held.spawn.label()
                ));
            }
        }
        let slotless: Vec<EnemyId> = self
            .spawns
            .iter()
            .filter(|(id, held)| {
                held.spawn.generator == generator
                    && held.stage == SpawnStage::InASlot
                    && !owned.contains(*id)
            })
            .map(|(id, _)| *id)
            .collect();
        for id in slotless {
            if self.spawns.get(&id).is_some_and(|held| held.over.is_some()) {
                self.spawns.remove(&id);
            } else if let Some(held) = self.spawns.get_mut(&id) {
                held.stage = SpawnStage::Heard;
                lines.push(format!(
                    "enemies: the slot of {} is gone before its enemy was made, it gets a new one",
                    id.label()
                ));
            }
        }
        lines
    }

    /// The enemies this copy made from the host's spawns, with their entity handles.
    pub fn made(&self) -> Vec<(EnemyId, u32)> {
        self.spawns
            .iter()
            .filter_map(|(id, held)| match held.stage {
                SpawnStage::Made { handle } => Some((*id, handle)),
                _ => None,
            })
            .collect()
    }

    /// The made enemies the host's game took away without a defeat: this copy takes them away too.
    pub fn made_and_gone_on_the_host(&self) -> Vec<(EnemyId, u32)> {
        self.spawns
            .iter()
            .filter_map(|(id, held)| match (held.stage, held.over) {
                (SpawnStage::Made { handle }, Some(OverOnTheHost::Gone)) => Some((*id, handle)),
                _ => None,
            })
            .collect()
    }

    /// The entity of a made enemy is gone here. Over on the host: forgotten. Still there on the
    /// host: it is made again when its generator next runs here.
    pub fn note_made_enemy_gone(&mut self, id: EnemyId) -> Option<String> {
        let held = self.spawns.get_mut(&id)?;
        if !matches!(held.stage, SpawnStage::Made { .. }) {
            return None;
        }
        if held.over.is_some() {
            self.spawns.remove(&id);
            return None;
        }
        held.stage = SpawnStage::Heard;
        Some(format!(
            "enemies: {} is gone here while the host still has it, it is made again when its generator next runs here",
            id.label()
        ))
    }

    fn of_generator(&self, generator: u32) -> impl Iterator<Item = &HeldSpawn> {
        self.spawns
            .values()
            .filter(move |held| held.spawn.generator == generator)
    }

    fn owners_of(
        &self,
        generator: u32,
        slots: &[GeneratorSlot],
    ) -> (Vec<Option<EnemyId>>, BTreeSet<EnemyId>) {
        let mut owned = BTreeSet::new();
        let mut owners: Vec<Option<EnemyId>> = vec![None; slots.len()];
        // Handles first: a made enemy's slot must not go to a waiting spawn with the same content.
        for by_handle in [true, false] {
            for (slot, owner) in slots.iter().zip(owners.iter_mut()) {
                if owner.is_some() {
                    continue;
                }
                let found = self
                    .of_generator(generator)
                    .find(|held| {
                        !owned.contains(&held.spawn.id())
                            && match held.stage {
                                SpawnStage::Made { handle } => {
                                    by_handle && slot.holds_a_made_enemy() && slot.handle == handle
                                }
                                SpawnStage::InASlot => !by_handle && slot_is_of(slot, &held.spawn),
                                SpawnStage::Heard | SpawnStage::NotMade => false,
                            }
                    })
                    .map(|held| held.spawn.id());
                if let Some(id) = found {
                    owned.insert(id);
                    *owner = Some(id);
                }
            }
        }
        (owners, owned)
    }
}

fn slot_is_of(slot: &GeneratorSlot, spawn: &EnemySpawn) -> bool {
    slot.state != GENERATOR_SLOT_STATE_FREED
        && slot.was_made_for(spawn.set_index, &spawn.position, &spawn.rotation)
}
