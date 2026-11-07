pub const PUPPET_MODEL_RESPAWN_INTERVAL_MS: u64 = 2000;

#[derive(Debug, Clone, Copy, Default)]
pub struct RespawnGate {
    last_spawn_ms: Option<u64>,
}

impl RespawnGate {
    pub fn may_spawn(&mut self, pack_loaded: bool, now_ms: u64) -> bool {
        if !pack_loaded {
            return false;
        }
        let waiting = self
            .last_spawn_ms
            .is_some_and(|at| now_ms.saturating_sub(at) < PUPPET_MODEL_RESPAWN_INTERVAL_MS);
        if waiting {
            return false;
        }
        self.last_spawn_ms = Some(now_ms);
        true
    }
}
