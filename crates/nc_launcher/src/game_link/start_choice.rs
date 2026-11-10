use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StartChoice {
    #[default]
    World,
    Slot,
    Title,
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
}

impl StartChoice {
    pub fn link_text(self) -> &'static str {
        match self {
            Self::World => "world",
            Self::Slot => "slot",
            Self::Title => "title",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::World => "Into the world",
            Self::Slot => "From a co-op save slot",
            Self::Title => "The title screen",
        }
    }
}
