use std::collections::BTreeMap;

use replicant_proto::EnemyId;

use crate::enemies::PLAYER_HIT_REPEAT_WINDOW_MS;

/// One enemy swing (attacker and hit id) counts once on a player's body inside a short window,
/// on the host before it sends the hit and on the guest before it applies it. Every repeat
/// renews the window, so a swing the game keeps handing over each frame stays one hit.
#[derive(Debug, Default)]
pub struct PlayerHitGuard {
    recent: BTreeMap<(EnemyId, u32), u64>,
}

impl PlayerHitGuard {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn counts(&mut self, now_ms: u64, attacker: EnemyId, hit_id: u32) -> bool {
        self.recent
            .retain(|_, at| now_ms.saturating_sub(*at) < PLAYER_HIT_REPEAT_WINDOW_MS);
        // Hit id 0 is not a swing counter (scripted or direct hits), so it is never a repeat.
        hit_id == 0 || self.recent.insert((attacker, hit_id), now_ms).is_none()
    }
}
