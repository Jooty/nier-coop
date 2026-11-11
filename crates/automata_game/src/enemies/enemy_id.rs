use serde::{Deserialize, Serialize};

/// What names one placed enemy the same way on both copies: the id of the entity layouts list
/// that placed it and the id of its entry there, both read by the game from its own data files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EnemyId {
    pub layout: u32,
    pub entry: u32,
}

/// One placed enemy as one copy sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnemySighting {
    pub id: EnemyId,
    pub object_id: u32,
    pub health: i32,
    pub max_health: i32,
}

impl EnemyId {
    pub fn label(self) -> String {
        format!("layout {:#X} entry {:#X}", self.layout, self.entry)
    }
}

impl EnemySighting {
    pub fn label(&self) -> String {
        format!(
            "{} (object {:#X}, health {}/{})",
            self.id.label(),
            self.object_id,
            self.health,
            self.max_health
        )
    }

    pub fn is_defeated(&self) -> bool {
        self.health < 1
    }
}
