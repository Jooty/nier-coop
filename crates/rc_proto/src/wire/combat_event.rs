use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CombatEvent {
    Downed,
    Revived,
    GameOver,
    Up,
    HostCheckpoint,
    HostGameOver,
}
