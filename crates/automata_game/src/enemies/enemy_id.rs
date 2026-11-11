use serde::{Deserialize, Serialize};

/// What names one enemy the same way on both copies. A placed enemy: the id of the entity layouts
/// list that placed it and the id of its entry there, both from the game's own data files. A
/// generator's enemy: the generator's action id from the data files and the host's count of its spawns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EnemyId {
    Placed { layout: u32, entry: u32 },
    Generated { generator: u32, serial: u32 },
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
        match self {
            Self::Placed { layout, entry } => format!("layout {layout:#X} entry {entry:#X}"),
            Self::Generated { generator, serial } => {
                format!("generator {generator:#X} spawn {serial}")
            }
        }
    }

    pub fn is_generated(self) -> bool {
        matches!(self, Self::Generated { .. })
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
