use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Character {
    TwoB,
    NineS,
    A2,
}

impl Character {
    pub const ALL: [Self; 3] = [Self::TwoB, Self::NineS, Self::A2];

    pub const fn object_id(self) -> u32 {
        match self {
            Self::TwoB => 0x10000,
            Self::A2 => 0x10100,
            Self::NineS => 0x10200,
        }
    }

    pub const fn launcher_id(self) -> &'static str {
        match self {
            Self::TwoB => "2b",
            Self::NineS => "9s",
            Self::A2 => "a2",
        }
    }

    pub fn from_launcher_id(id: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|character| character.launcher_id() == id)
    }

    pub fn from_object_id(id: u32) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|character| character.object_id() == id)
    }
}
