use std::ops::RangeInclusive;

use nc_wire::{MAX_SESSION_SLOTS, STORY_MODE, SessionRules};

pub const FREE_ROAM_MODE: u8 = 1;
const CAMPAIGN_PLAYERS: u8 = 2;
const FREE_ROAM_MIN_PLAYERS: u8 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SessionMode {
    /// Exactly two players; the partner is the engine's companion slot body.
    #[default]
    Campaign,
    /// Two to sixteen players and no story rule.
    FreeRoam,
}

impl SessionMode {
    /// Mode 0 is the campaign, mode 1 is free roam, any other mode is treated as free roam.
    pub fn from_rules(rules: SessionRules) -> Self {
        if rules.mode == STORY_MODE {
            Self::Campaign
        } else {
            Self::FreeRoam
        }
    }

    pub fn is_a_known_mode(mode: u8) -> bool {
        mode == STORY_MODE || mode == FREE_ROAM_MODE
    }

    pub fn player_range(self) -> RangeInclusive<u8> {
        match self {
            Self::Campaign => CAMPAIGN_PLAYERS..=CAMPAIGN_PLAYERS,
            Self::FreeRoam => FREE_ROAM_MIN_PLAYERS..=MAX_SESSION_SLOTS,
        }
    }

    pub fn story_rules_apply(self) -> bool {
        self == Self::Campaign
    }
}
