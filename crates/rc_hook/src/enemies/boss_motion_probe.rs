use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::{Duration, Instant};

use rc_game::enemies::EnemySighting;
use rc_game::npcs::motion_layers_text;
use rc_proto::EnemyId;

use crate::boss_messages::is_boss;
use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::enemies::enemy_id_text;
use crate::memory::InProcessMemory;

const BOSS_MOTION_LOG_INTERVAL: Duration = Duration::from_secs(2);

static LAST_LOGGED: LazyLock<Mutex<HashMap<u64, Instant>>> = LazyLock::new(Mutex::default);

pub(super) fn log_boss_motion_layers(
    build: &VerifiedBuild,
    role: &str,
    seen: &[(EnemyId, EnemySighting)],
) {
    let memory = InProcessMemory::new(build);
    for (id, sighting) in seen {
        if !is_boss(*id) || !due(sighting.actor) {
            continue;
        }
        log_line(&format!(
            "boss motion ({role}): {} {}",
            enemy_id_text(*id),
            motion_layers_text(&memory, sighting.actor)
        ));
    }
}

fn due(actor: u64) -> bool {
    let mut last = LAST_LOGGED.lock().unwrap_or_else(PoisonError::into_inner);
    let now = Instant::now();
    if last
        .get(&actor)
        .is_some_and(|at| now.saturating_duration_since(*at) < BOSS_MOTION_LOG_INTERVAL)
    {
        return false;
    }
    last.insert(actor, now);
    true
}
