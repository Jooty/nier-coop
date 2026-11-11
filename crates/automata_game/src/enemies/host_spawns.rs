use std::collections::BTreeMap;

use crate::enemies::{
    EnemyId, EnemySpawn, GENERATOR_STATES_IN_ONE_MESSAGE, GeneratorState, GeneratorStates,
    MOST_GENERATOR_STATES_TOLD, MOST_HOST_SPAWNS_HELD,
};

/// A spawn a copy saw its own generator make.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObservedSpawn {
    pub generator: u32,
    pub set_index: u32,
    pub object_id: u32,
    pub level: Option<i32>,
    pub uses_the_second_set_values: bool,
    pub object_read_variant: u32,
    pub set_values: [i32; 4],
    pub handle: u32,
    pub position: [f32; 4],
    pub rotation: [f32; 4],
}

/// The host's book of its generators' spawns: the serial each one got, the ones still there, and
/// the generator states the guest was told.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HostSpawns {
    serials_given: BTreeMap<u32, u32>,
    living: BTreeMap<EnemyId, (EnemySpawn, u32)>,
    states_told: BTreeMap<u32, GeneratorState>,
}

impl HostSpawns {
    /// Gives the spawn the next serial of its generator; a serial is never given twice in a session.
    pub fn note(&mut self, observed: ObservedSpawn) -> EnemySpawn {
        let given = self.serials_given.entry(observed.generator).or_insert(0);
        *given = given.wrapping_add(1);
        let spawn = EnemySpawn {
            generator: observed.generator,
            serial: *given,
            set_index: observed.set_index,
            object_id: observed.object_id,
            level: observed.level,
            uses_the_second_set_values: observed.uses_the_second_set_values,
            object_read_variant: observed.object_read_variant,
            set_values: observed.set_values,
            position: observed.position,
            rotation: observed.rotation,
        };
        if self.living.len() >= MOST_HOST_SPAWNS_HELD {
            self.living.pop_first();
        }
        self.living.insert(spawn.id(), (spawn, observed.handle));
        spawn
    }

    pub fn living(&self) -> Vec<(EnemyId, u32)> {
        self.living
            .iter()
            .map(|(id, (_, handle))| (*id, *handle))
            .collect()
    }

    /// True when the spawn was held: its entity is gone and the guest is to be told.
    pub fn note_gone(&mut self, id: EnemyId) -> bool {
        self.living.remove(&id).is_some()
    }

    /// What a newly linked guest needs: every spawn still there.
    pub fn spawns_still_there(&self) -> Vec<EnemySpawn> {
        self.living.values().map(|(spawn, _)| *spawn).collect()
    }

    /// The states that differ from what the guest holds, in messages; a generator the guest was
    /// told nothing about counts as waiting with nothing made.
    pub fn states_to_tell(&mut self, current: &[GeneratorState]) -> Vec<GeneratorStates> {
        if self.states_told.len() > MOST_GENERATOR_STATES_TOLD {
            self.states_told
                .retain(|generator, _| current.iter().any(|state| state.generator == *generator));
        }
        let changed: Vec<GeneratorState> = current
            .iter()
            .filter(|state| {
                let told = self
                    .states_told
                    .get(&state.generator)
                    .copied()
                    .unwrap_or(GeneratorState::untold(state.generator));
                told != **state
            })
            .copied()
            .collect();
        for state in &changed {
            self.states_told.insert(state.generator, *state);
        }
        changed
            .chunks(GENERATOR_STATES_IN_ONE_MESSAGE)
            .map(|chunk| GeneratorStates {
                states: chunk.to_vec(),
            })
            .collect()
    }

    pub fn forget_what_the_guest_was_told(&mut self) {
        self.states_told.clear();
    }
}
