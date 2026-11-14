use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::Instant;

use nt_hook::line_budget::LineBudget;
use replicant_game::enemies::{
    BOSS_HP_LOG_INTERVAL_MS, BossFight, BossFightChange, EnemySighting, boss_fight_hp_text,
    boss_hp_text,
};
use replicant_proto::EnemyId;

use crate::diagnostics::log_line;
use crate::enemies::enemy_id_text;

static BOSS_FIGHT: Mutex<BossFight> = Mutex::new(BossFight::new());
static BOSS_HP_CLOCK: Mutex<LineBudget> = Mutex::new(LineBudget::new());
static BOSS_HP_EPOCH: LazyLock<Instant> = LazyLock::new(Instant::now);

#[cfg(feature = "probes")]
pub fn is_boss_fight_on() -> bool {
    BOSS_FIGHT
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .boss()
        .is_some()
}

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
    let (change, boss) = {
        let mut fight = BOSS_FIGHT.lock().unwrap_or_else(PoisonError::into_inner);
        let change = fight.observe(phase, &enemies);
        (change, fight.boss())
    };
    match change {
        Some(BossFightChange::On(boss)) => log_line(&format!(
            "boss fight: on, boss is {} (max hp {})",
            enemy_id_text(boss.id),
            boss.max_hp
        )),
        Some(BossFightChange::Off) => log_line("boss fight: off (phase changed)"),
        None => {}
    }
    log_boss_hp(change, boss, &enemies);
}

fn log_boss_hp(
    change: Option<BossFightChange>,
    boss: Option<replicant_game::enemies::BossPick>,
    enemies: &[(EnemyId, i32, i32)],
) {
    let mut clock = BOSS_HP_CLOCK.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(boss) = boss else {
        clock.reset();
        return;
    };
    let Some(hp) = enemies
        .iter()
        .find(|(id, _, _)| *id == boss.id)
        .map(|(_, hp, _)| *hp)
    else {
        return;
    };
    let now_ms = u64::try_from(BOSS_HP_EPOCH.elapsed().as_millis()).unwrap_or(u64::MAX);
    if matches!(change, Some(BossFightChange::On(_))) {
        clock.reset();
        clock.due(now_ms, BOSS_HP_LOG_INTERVAL_MS);
        log_line(&boss_fight_hp_text(hp, boss.max_hp));
    } else if clock.due(now_ms, BOSS_HP_LOG_INTERVAL_MS) {
        log_line(&boss_hp_text(hp, boss.max_hp));
    }
}
