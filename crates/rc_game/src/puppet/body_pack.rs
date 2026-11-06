pub const PACK_SETTLE_AFTER_FOUND_MS: u64 = 1500;
pub const PACK_REQUEST_RETRY_MS: u64 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyPackStep {
    Request,
    Wait,
    Spawn,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BodyPackLoad {
    requested_at_ms: Option<u64>,
    found_at_ms: Option<u64>,
}

impl BodyPackLoad {
    pub const fn new() -> Self {
        Self {
            requested_at_ms: None,
            found_at_ms: None,
        }
    }

    pub fn step(&mut self, pack_found: bool, now_ms: u64) -> BodyPackStep {
        if !pack_found {
            self.found_at_ms = None;
            let waiting = self
                .requested_at_ms
                .is_some_and(|at| now_ms.saturating_sub(at) < PACK_REQUEST_RETRY_MS);
            if waiting {
                return BodyPackStep::Wait;
            }
            self.requested_at_ms = Some(now_ms);
            return BodyPackStep::Request;
        }
        if self.requested_at_ms.is_none() {
            return BodyPackStep::Spawn;
        }
        let found_at = *self.found_at_ms.get_or_insert(now_ms);
        if now_ms.saturating_sub(found_at) >= PACK_SETTLE_AFTER_FOUND_MS {
            BodyPackStep::Spawn
        } else {
            BodyPackStep::Wait
        }
    }

    pub fn requested_by_us(&self) -> bool {
        self.requested_at_ms.is_some()
    }
}

pub fn release_requested_pack_wanted(requested_by_us: bool, local_player_wears_pack: bool) -> bool {
    requested_by_us && !local_player_wears_pack
}
