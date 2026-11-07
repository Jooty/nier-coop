use serde::{Deserialize, Serialize};

use crate::wire::WireError;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PausedHits {
    pub paused_players_can_be_hit: bool,
}

impl PausedHits {
    pub fn validate(&self) -> Result<(), WireError> {
        Ok(())
    }
}
