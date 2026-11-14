use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameReadiness {
    #[default]
    NotLinked,
    Loading,
    InPlay,
}

impl GameReadiness {
    pub fn is_in_play(self) -> bool {
        self == Self::InPlay
    }
}
