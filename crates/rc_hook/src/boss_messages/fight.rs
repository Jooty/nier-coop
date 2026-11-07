use std::sync::{Mutex, PoisonError};

use rc_game::enemies::{BossFight, BossFightChange, EnemySighting};
use rc_proto::EnemyId;

use crate::diagnostics::log_line;
use crate::enemies::enemy_id_text;

static BOSS_FIGHT: Mutex<BossFight> = Mutex::new(BossFight::new());

pub fn raise(phase: u16) {
    let mut fight = BOSS_FIGHT.lock().unwrap_or_else(PoisonError::into_inner);
    fight.raise(phase);
}

pub fn is_boss(id: EnemyId) -> bool {
    BOSS_FIGHT
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .is_boss(id)
}

pub fn observe(phase: u16, seen: &[(EnemyId, EnemySighting)]) {
    let enemies: Vec<(EnemyId, i32, i32)> = seen
        .iter()
        .map(|(id, sighting)| (*id, sighting.hp, sighting.max_hp))
        .collect();
    let change = BOSS_FIGHT
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .observe(phase, &enemies);
    match change {
        Some(BossFightChange::On(boss)) => log_line(&format!(
            "boss fight: on, boss is {} (max hp {})",
            enemy_id_text(boss.id),
            boss.max_hp
        )),
        Some(BossFightChange::Off) => log_line("boss fight: off (phase changed)"),
        None => {}
    }
}
