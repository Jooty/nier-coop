use serde::{Deserialize, Serialize};

pub const DEFAULT_ENEMY_HEALTH_PERCENT: u16 = 60;
pub const MAX_ENEMY_HEALTH_PERCENT: u16 = 300;
pub const DEFAULT_ENEMY_DAMAGE_PERCENT: u16 = 10;
pub const MAX_ENEMY_DAMAGE_PERCENT: u16 = 50;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionOptions {
    pub show_names: bool,
    pub show_party_frame: bool,
    pub show_distance: bool,
    pub show_ping: bool,
    pub enemy_health_percent: u16,
    pub enemy_damage_percent: u16,
}

impl Default for SessionOptions {
    fn default() -> Self {
        Self {
            show_names: true,
            show_party_frame: true,
            show_distance: false,
            show_ping: false,
            enemy_health_percent: DEFAULT_ENEMY_HEALTH_PERCENT,
            enemy_damage_percent: DEFAULT_ENEMY_DAMAGE_PERCENT,
        }
    }
}

impl SessionOptions {
    pub fn clamped(self) -> Self {
        Self {
            enemy_health_percent: self.enemy_health_percent.min(MAX_ENEMY_HEALTH_PERCENT),
            enemy_damage_percent: self.enemy_damage_percent.min(MAX_ENEMY_DAMAGE_PERCENT),
            ..self
        }
    }
}
