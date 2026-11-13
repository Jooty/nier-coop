use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StartChoice {
    #[default]
    World,
    Slot,
    Title,
    NewGame,
    Continue,
}

pub const START_CHOICES: [StartChoice; 3] =
    [StartChoice::World, StartChoice::Slot, StartChoice::Title];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct StartSetting {
    #[serde(default)]
    pub choice: StartChoice,
    #[serde(default)]
    pub index: u8,
    #[serde(default)]
    pub slot: u8,
    #[serde(default)]
    pub story_allowed: bool,
}

impl StartSetting {
    pub fn story_link_text(self) -> &'static str {
        if self.story_allowed { "allow" } else { "hold" }
    }
}

impl StartChoice {
    pub fn link_text(self) -> &'static str {
        match self {
            Self::World => "world",
            Self::Slot => "slot",
            Self::Title => "title",
            Self::NewGame => "new_game",
            Self::Continue => "continue",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::World => "Into the world",
            Self::Slot => "From a co-op save slot",
            Self::Title => "The title screen",
            Self::NewGame => "A new game of the campaign",
            Self::Continue => "The newest co-op save of the campaign",
        }
    }
}
