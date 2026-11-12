use crate::character::Character;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartMode {
    World,
    Slot,
    Title,
}

impl StartMode {
    pub fn from_text(text: &str) -> Option<Self> {
        match text {
            "world" => Some(Self::World),
            "slot" => Some(Self::Slot),
            "title" => Some(Self::Title),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::World => "world",
            Self::Slot => "slot",
            Self::Title => "title",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartConfig {
    pub mode: StartMode,
    pub index: usize,
    pub slot: u32,
    pub hero: Character,
    pub partner: Character,
    pub route_lead: Character,
}
