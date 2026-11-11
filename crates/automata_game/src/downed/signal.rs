use serde::{Deserialize, Serialize};

use crate::downed::REVIVE_PERCENT_WHEN_DONE;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DownedPhase {
    #[default]
    Up,
    Downed,
    /// The game's own defeat has the hero: nobody can revive it and it revives nobody.
    Defeated,
}

/// What one player tells the other about going down and reviving, in every state message.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownedSignal {
    /// The sender has downed and revive switched on and will revive.
    pub revives: bool,
    pub phase: DownedPhase,
    /// Counts the sender's own downs, wrapping; names the down a revive is for.
    pub times_downed: u8,
    /// Which of the receiver's downs the sender is reviving.
    pub reviving_down: u8,
    /// 100 says the revive is done; the sender holds it until the receiver is up.
    pub reviving_percent: u8,
}

impl DownedSignal {
    pub fn is_valid(&self) -> bool {
        self.reviving_percent <= REVIVE_PERCENT_WHEN_DONE
    }
}
