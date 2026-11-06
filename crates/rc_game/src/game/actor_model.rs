use crate::offsets::ACTOR_MODEL_TEXT_LEN;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActorModel {
    #[default]
    NierB,
    NierT,
    NierF,
    NierY,
    KaineE,
}

impl ActorModel {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "nierB" => Some(Self::NierB),
            "nierT" => Some(Self::NierT),
            "nierF" => Some(Self::NierF),
            "nierY" => Some(Self::NierY),
            "kaineE" => Some(Self::KaineE),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::NierB => "nierB",
            Self::NierT => "nierT",
            Self::NierF => "nierF",
            Self::NierY => "nierY",
            Self::KaineE => "kaineE",
        }
    }

    pub fn bytes(self) -> [u8; ACTOR_MODEL_TEXT_LEN] {
        let mut bytes = [0_u8; ACTOR_MODEL_TEXT_LEN];
        let name = self.name().as_bytes();
        bytes[..name.len()].copy_from_slice(name);
        bytes
    }
}
