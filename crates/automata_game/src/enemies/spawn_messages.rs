use serde::{Deserialize, Serialize};

use crate::enemies::{
    EnemyId, GENERATOR_STATES_IN_ONE_MESSAGE, GeneratorPhase, GeneratorRunCounts,
    HIGHEST_ENEMY_LEVEL_ON_THE_WIRE, HIGHEST_SPAWN_COORDINATE_ABS, LOWEST_ENEMY_LEVEL_ON_THE_WIRE,
    MOST_SETS_OF_ONE_GENERATOR,
};

/// One spawn the host's generator decided: all the guest needs to make the same enemy with its
/// own generator. No level means the host's game was not seen picking one.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EnemySpawn {
    pub generator: u32,
    pub serial: u32,
    pub set_index: u32,
    pub object_id: u32,
    pub level: Option<i32>,
    pub uses_the_second_set_values: bool,
    pub object_read_variant: u32,
    pub set_values: [i32; 4],
    pub position: [f32; 4],
    pub rotation: [f32; 4],
}

/// Where one of the host's generators stands and how many enemies its run has made so far; the
/// run's counts only while the host's `enemy_counts_follow_the_host` is on and the generator is started.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratorState {
    pub generator: u32,
    pub phase: GeneratorPhase,
    pub spawned: u32,
    pub counts: Option<GeneratorRunCounts>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratorStates {
    pub states: Vec<GeneratorState>,
}

impl EnemySpawn {
    pub fn id(&self) -> EnemyId {
        EnemyId::Generated {
            generator: self.generator,
            serial: self.serial,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.generator != 0
            && self.set_index < MOST_SETS_OF_ONE_GENERATOR
            && self.level.is_none_or(|level| {
                (LOWEST_ENEMY_LEVEL_ON_THE_WIRE..=HIGHEST_ENEMY_LEVEL_ON_THE_WIRE).contains(&level)
            })
            && self
                .position
                .iter()
                .chain(&self.rotation)
                .all(|value| value.is_finite() && value.abs() <= HIGHEST_SPAWN_COORDINATE_ABS)
    }

    pub fn label(&self) -> String {
        let level = self
            .level
            .map_or("no level seen".to_owned(), |level| format!("level {level}"));
        format!(
            "{} (object {:#X} from set {}{}, {level}, at {:.1} {:.1} {:.1})",
            self.id().label(),
            self.object_id,
            self.set_index,
            if self.uses_the_second_set_values {
                " with its second values"
            } else {
                ""
            },
            self.position[0],
            self.position[1],
            self.position[2]
        )
    }
}

impl GeneratorState {
    /// What a copy holds for a generator it was told nothing about.
    pub fn untold(generator: u32) -> Self {
        Self {
            generator,
            phase: GeneratorPhase::Waiting,
            spawned: 0,
            counts: None,
        }
    }

    pub fn label(&self) -> String {
        format!(
            "generator {:#X} is {} with {} made{}",
            self.generator,
            self.phase.label(),
            self.spawned,
            self.counts
                .map_or(String::new(), |counts| format!(", {}", counts.label()))
        )
    }
}

impl GeneratorStates {
    pub fn is_valid(&self) -> bool {
        self.states.len() <= GENERATOR_STATES_IN_ONE_MESSAGE
            && self
                .states
                .iter()
                .all(|state| state.counts.is_none_or(|counts| counts.is_valid()))
    }
}
