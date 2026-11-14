use nc_wire::{MAX_SESSION_SLOTS, STORY_MODE, SessionRules};

pub const CAMPAIGN_PLAYER_COUNT: u8 = 2;
pub const FREE_ROAM_MODE: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameMode {
    pub id: u8,
    pub label: &'static str,
    pub max_players: u8,
}

pub const CO_OP_CAMPAIGN: GameMode = GameMode {
    id: STORY_MODE,
    label: "Co-op campaign",
    max_players: CAMPAIGN_PLAYER_COUNT,
};

pub const FREE_ROAM: GameMode = GameMode {
    id: FREE_ROAM_MODE,
    label: "Free roam",
    max_players: MAX_SESSION_SLOTS,
};

impl GameMode {
    pub fn is_campaign(&self) -> bool {
        self.id == STORY_MODE
    }

    pub fn rules(&self) -> SessionRules {
        SessionRules {
            mode: self.id,
            max_players: self.max_players,
            map: 0,
        }
    }
}
