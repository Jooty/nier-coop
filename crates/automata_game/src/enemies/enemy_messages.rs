use serde::{Deserialize, Serialize};

use crate::enemies::{
    ENEMIES_IN_ONE_ROSTER_PART, ENEMY_HITS_IN_ONE_MESSAGE, EnemyId, EnemySighting,
    HIGHEST_ENEMY_HEALTH_ON_THE_WIRE,
};

/// A slice of one copy's list of placed enemies; `last` closes the round.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnemyRosterPart {
    pub round: u32,
    pub last: bool,
    pub enemies: Vec<EnemySighting>,
}

/// Health one player's own hit took from an enemy on that player's copy, with that copy's full health.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnemyHit {
    pub id: EnemyId,
    pub amount: u32,
    pub max_health: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnemyHits {
    pub hits: Vec<EnemyHit>,
}

impl EnemyRosterPart {
    pub fn is_valid(&self) -> bool {
        self.enemies.len() <= ENEMIES_IN_ONE_ROSTER_PART
            && self.enemies.iter().all(|enemy| {
                health_is_on_the_scale(enemy.health) && health_is_on_the_scale(enemy.max_health)
            })
    }
}

impl EnemyHits {
    pub fn is_valid(&self) -> bool {
        self.hits.len() <= ENEMY_HITS_IN_ONE_MESSAGE
            && self.hits.iter().all(|hit| {
                hit.max_health >= 1
                    && health_is_on_the_scale(hit.max_health)
                    && i32::try_from(hit.amount).is_ok_and(health_is_on_the_scale)
            })
    }
}

fn health_is_on_the_scale(health: i32) -> bool {
    (0..=HIGHEST_ENEMY_HEALTH_ON_THE_WIRE).contains(&health)
}
