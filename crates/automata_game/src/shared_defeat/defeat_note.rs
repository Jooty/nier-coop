use serde::{Deserialize, Serialize};

use crate::shared_defeat::{DefeatWay, GameOverChoice, RestorePlace};

/// What one copy tells the other about its defeat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DefeatNote {
    /// The sender's flow took this way on; `restore` is its restore point's place, if it has one.
    Way {
        way: DefeatWay,
        restore: Option<RestorePlace>,
    },
    /// The first choice made in the sender's own game-over menu by its player.
    MenuChoice(GameOverChoice),
}

impl DefeatNote {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Way { restore, .. } => restore.as_ref().is_none_or(RestorePlace::is_valid),
            Self::MenuChoice(_) => true,
        }
    }
}
