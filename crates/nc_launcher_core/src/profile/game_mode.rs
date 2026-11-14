use nc_wire::{MAX_SESSION_SLOTS, STORY_MODE, SessionRules};

pub const CAMPAIGN_PLAYER_COUNT: u8 = 2;
pub const FREE_ROAM_MODE: u8 = 1;
pub const HIDE_AND_SEEK_MODE: u8 = 2;
pub const PROP_HUNT_MODE: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeAvailability {
    Playable,
    Planned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameMode {
    pub id: u8,
    pub label: &'static str,
    pub max_players: u8,
    pub availability: ModeAvailability,
}

pub const CO_OP_CAMPAIGN: GameMode = GameMode {
    id: STORY_MODE,
    label: "Co-op campaign",
    max_players: CAMPAIGN_PLAYER_COUNT,
    availability: ModeAvailability::Playable,
};

pub const FREE_ROAM: GameMode = GameMode {
    id: FREE_ROAM_MODE,
    label: "Free roam",
    max_players: MAX_SESSION_SLOTS,
    availability: ModeAvailability::Playable,
};

pub const HIDE_AND_SEEK: GameMode = GameMode {
    id: HIDE_AND_SEEK_MODE,
    label: "Hide and seek",
    max_players: MAX_SESSION_SLOTS,
    availability: ModeAvailability::Planned,
};

pub const PROP_HUNT: GameMode = GameMode {
    id: PROP_HUNT_MODE,
    label: "Prop hunt",
    max_players: MAX_SESSION_SLOTS,
    availability: ModeAvailability::Planned,
};

impl GameMode {
    pub fn pickable(&self) -> bool {
        self.availability == ModeAvailability::Playable
    }

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
