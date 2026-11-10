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

pub const BOOK_SPAWN_FIRST_RETRY_MS: u64 = 500;
pub const BOOK_SPAWN_LONGEST_RETRY_MS: u64 = 8000;
pub const BOOK_PACK_REREQUEST_TIMEOUT_MS: u64 = 5000;
pub const BOOK_SPAWN_LOG_EVERY_ATTEMPTS: u32 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookSpawnStep {
    Wait,
    RequestPack { rerequest: bool },
    TrySpawn { attempt: u32, log: bool },
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BookSpawnPlanner {
    pack_requested_at_ms: Option<u64>,
    attempts: u32,
    next_attempt_at_ms: u64,
}

impl BookSpawnPlanner {
    pub fn next(&mut self, pack_resident: bool, loading: bool, now_ms: u64) -> BookSpawnStep {
        if loading {
            return BookSpawnStep::Wait;
        }
        if !pack_resident {
            let rerequest = self.pack_requested_at_ms.is_some();
            let due = self
                .pack_requested_at_ms
                .is_none_or(|at| now_ms.saturating_sub(at) >= BOOK_PACK_REREQUEST_TIMEOUT_MS);
            if !due {
                return BookSpawnStep::Wait;
            }
            self.pack_requested_at_ms = Some(now_ms);
            return BookSpawnStep::RequestPack { rerequest };
        }
        self.pack_requested_at_ms = None;
        if now_ms < self.next_attempt_at_ms {
            return BookSpawnStep::Wait;
        }
        self.attempts = self.attempts.saturating_add(1);
        let backoff = BOOK_SPAWN_FIRST_RETRY_MS
            .saturating_mul(1_u64 << (self.attempts - 1).min(16))
            .min(BOOK_SPAWN_LONGEST_RETRY_MS);
        self.next_attempt_at_ms = now_ms.saturating_add(backoff);
        BookSpawnStep::TrySpawn {
            attempt: self.attempts,
            log: self.attempts == 1 || self.attempts.is_multiple_of(BOOK_SPAWN_LOG_EVERY_ATTEMPTS),
        }
    }

    pub fn forget_pack_request(&mut self) {
        self.pack_requested_at_ms = None;
    }

    pub fn spawned(&mut self) {
        *self = Self::default();
    }
}
