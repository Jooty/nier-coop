use rc_proto::BULLET_EFFECT_SLOTS;

use crate::enemy_bullets::MAX_ENEMY_BULLET_KINDS_LOGGED;

/// The bullet kinds (effect id sets) already logged, so each kind is logged once.
#[derive(Debug, Default)]
pub struct EnemyBulletKinds {
    seen: Vec<[i32; BULLET_EFFECT_SLOTS]>,
}

impl EnemyBulletKinds {
    pub fn first_sighting(&mut self, kind: [i32; BULLET_EFFECT_SLOTS]) -> bool {
        if self.seen.contains(&kind) || self.seen.len() >= MAX_ENEMY_BULLET_KINDS_LOGGED {
            return false;
        }
        self.seen.push(kind);
        true
    }
}
