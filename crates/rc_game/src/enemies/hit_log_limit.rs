use std::collections::HashMap;

use rc_proto::EnemyId;

const HIT_LOG_GAP_MS: u64 = 400;

#[derive(Debug, Clone, Default)]
pub struct EnemyHitLogLimiter {
    last_logged_ms: HashMap<EnemyId, u64>,
}

impl EnemyHitLogLimiter {
    pub fn allow(&mut self, now_ms: u64, attacker: EnemyId) -> bool {
        let due = self
            .last_logged_ms
            .get(&attacker)
            .is_none_or(|at| now_ms.saturating_sub(*at) >= HIT_LOG_GAP_MS);
        if due {
            self.last_logged_ms.insert(attacker, now_ms);
        }
        due
    }
}
