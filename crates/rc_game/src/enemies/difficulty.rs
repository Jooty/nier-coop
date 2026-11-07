use std::collections::HashMap;

use rc_proto::{MAX_ENEMY_DAMAGE_PERCENT, MAX_ENEMY_HEALTH_PERCENT};

pub const EXTRA_PLAYERS_WITH_ONE_GUEST: u32 = 1;
const PERCENT_DIVISOR: i64 = 100;
const PERCENT_DIVISOR_FLOAT: f32 = 100.0;
const SCALE_WRITE_TOLERANCE: f32 = 1e-5;

pub type ScaledEnemyKey = (u64, i32);

#[derive(Debug, Clone, Copy)]
struct ScaleEntry {
    original: f32,
    written: f32,
}

#[derive(Debug, Default)]
pub struct DamageTakenLedger {
    entries: HashMap<ScaledEnemyKey, ScaleEntry>,
}

impl DamageTakenLedger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn writes(
        &mut self,
        seen: &[(ScaledEnemyKey, f32)],
        factor: f32,
    ) -> Vec<(ScaledEnemyKey, f32)> {
        let mut writes = Vec::new();
        let mut entries = HashMap::with_capacity(seen.len());
        for (key, current) in seen {
            let original = match self.entries.get(key) {
                // Our own earlier write is still there; anything else is the game's new value.
                Some(entry) if same(entry.written, *current) => entry.original,
                _ => *current,
            };
            let wanted = original * factor;
            if !same(wanted, *current) {
                writes.push((*key, wanted));
            }
            entries.insert(
                *key,
                ScaleEntry {
                    original,
                    written: wanted,
                },
            );
        }
        self.entries = entries;
        writes
    }

    pub fn restores(&mut self) -> Vec<(ScaledEnemyKey, f32)> {
        let restores = self
            .entries
            .iter()
            .filter(|(_, entry)| !same(entry.original, entry.written))
            .map(|(key, entry)| (*key, entry.original))
            .collect();
        self.entries.clear();
        restores
    }

    /// The scale last written for `key`, when it differs from the enemy's own. An enemy that
    /// puts its own scale back between scans (a boss does) needs it again right before a hit.
    pub fn wanted(&self, key: ScaledEnemyKey) -> Option<f32> {
        self.entries
            .get(&key)
            .filter(|entry| !same(entry.original, entry.written))
            .map(|entry| entry.written)
    }

    pub fn scaled_count(&self) -> usize {
        self.entries
            .values()
            .filter(|entry| !same(entry.original, entry.written))
            .count()
    }
}

pub fn damage_taken_factor(health_percent: u16, extra_players: u32) -> f32 {
    let bonus = health_percent.min(MAX_ENEMY_HEALTH_PERCENT);
    1.0 / (1.0 + f32::from(bonus) * extra_players as f32 / PERCENT_DIVISOR_FLOAT)
}

pub fn damage_dealt_bonus_percent(damage_percent: u16, extra_players: u32) -> u32 {
    u32::from(damage_percent.min(MAX_ENEMY_DAMAGE_PERCENT)) * extra_players
}

pub fn scaled_hp_damage(hp_damage: i32, bonus_percent: u32) -> i32 {
    if hp_damage <= 0 || bonus_percent == 0 {
        return hp_damage;
    }
    let scaled = (i64::from(hp_damage) * (PERCENT_DIVISOR + i64::from(bonus_percent))
        + PERCENT_DIVISOR / 2)
        / PERCENT_DIVISOR;
    i32::try_from(scaled).unwrap_or(i32::MAX)
}

fn same(left: f32, right: f32) -> bool {
    (left - right).abs() <= SCALE_WRITE_TOLERANCE
}
