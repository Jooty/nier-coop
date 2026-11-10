use std::collections::BTreeMap;

use replicant_proto::{EnemyHit, EnemyId};

use crate::enemies::GUEST_HIT_REPEAT_WINDOW_MS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitVerdict {
    Apply,
    WrongPhase,
    Repeat,
    NoDamage,
}

/// The host's check on the guest's hit reports: same phase, some damage, and one swing (hit id)
/// lands on one enemy once, as the game's own hit-once latch does.
#[derive(Debug, Default)]
pub struct GuestHitGuard {
    recent: BTreeMap<(EnemyId, u32), u64>,
}

impl GuestHitGuard {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn judge(&mut self, now_ms: u64, phase: u16, hit: &EnemyHit) -> HitVerdict {
        self.recent
            .retain(|_, at| now_ms.saturating_sub(*at) < GUEST_HIT_REPEAT_WINDOW_MS);
        if hit.phase != phase {
            return HitVerdict::WrongPhase;
        }
        if hit.hp_damage == 0 && hit.second_gauge == 0 && hit.third_gauge == 0 {
            return HitVerdict::NoDamage;
        }
        // Hit id 0 is not a swing counter (scripted or direct hits), so it is never a repeat.
        if hit.hit_id != 0 && self.recent.insert((hit.id, hit.hit_id), now_ms).is_some() {
            return HitVerdict::Repeat;
        }
        HitVerdict::Apply
    }
}
