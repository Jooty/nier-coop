use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::Instant;

use replicant_game::enemies::{AttackRecord, BossHitLog, BossHitPlayer, ca_gauge_hit_text};
use replicant_proto::EnemyHit;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::enemies::enemy_id_text;
use crate::memory::InProcessMemory;

static BOSS_HITS: LazyLock<Mutex<BossHitLog>> = LazyLock::new(Mutex::default);
static BOSS_HIT_CLOCK: LazyLock<Instant> = LazyLock::new(Instant::now);

pub(super) fn log_host_boss_hit(
    build: &VerifiedBuild,
    victim: u64,
    before: i32,
    after: i32,
    attack: &AttackRecord,
) {
    let ca = ca_gauge_hit_text(&InProcessMemory::new(build), victim);
    let line = format!(
        "boss hit (host): hp {before} -> hp {after}, kind {}, class {}, flags {:#x}, gauges {}/{}, {ca}",
        attack.attack_kind,
        attack.source_class,
        attack.flags,
        attack.second_gauge,
        attack.third_gauge
    );
    log_hit(
        BossHitPlayer::Host,
        attack.attack_kind,
        before - after,
        &line,
    );
}

pub(super) fn log_guest_boss_hit(
    build: &VerifiedBuild,
    victim: u64,
    hit: &EnemyHit,
    applied_as: u32,
    before: i32,
    after: i32,
    kind: i32,
    source_class: Option<i32>,
) {
    let ca = ca_gauge_hit_text(&InProcessMemory::new(build), victim);
    let class = source_class.map_or_else(|| "unsent".to_owned(), |class| class.to_string());
    let line = format!(
        "boss hit (guest): sent {}, applied as {applied_as}, hp {before} -> hp {after}, kind {kind}, class {class}, flags {:#x} ({}), {ca}",
        hit.hp_damage,
        hit.flags,
        enemy_id_text(hit.id)
    );
    log_hit(BossHitPlayer::Guest, kind, before - after, &line);
}

fn log_hit(player: BossHitPlayer, kind: i32, damage: i32, line: &str) {
    let now_ms = u64::try_from(BOSS_HIT_CLOCK.elapsed().as_millis()).unwrap_or(u64::MAX);
    let mut log = BOSS_HITS.lock().unwrap_or_else(PoisonError::into_inner);
    if log.allow_line(now_ms) {
        log_line(line);
    }
    if let Some(summary) = log.record(now_ms, player, kind, damage) {
        log_line(&summary);
    }
}
