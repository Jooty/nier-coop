use std::fmt;

use rc_game::GameError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PuppetError {
    Game(GameError),
    CreateReturnedNull,
    TooManyPuppets,
    UnknownPuppet(u32),
    TransformSetterMissing,
    NameHasNul,
}

impl From<GameError> for PuppetError {
    fn from(error: GameError) -> Self {
        Self::Game(error)
    }
}

impl From<rc_game::MemoryError> for PuppetError {
    fn from(error: rc_game::MemoryError) -> Self {
        Self::Game(GameError::Memory(error))
    }
}

impl fmt::Display for PuppetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Game(error) => fmt::Display::fmt(error, formatter),
            Self::CreateReturnedNull => write!(formatter, "the game did not create the actor"),
            Self::TooManyPuppets => write!(formatter, "puppet limit reached"),
            Self::UnknownPuppet(id) => write!(formatter, "no puppet with id {id}"),
            Self::TransformSetterMissing => {
                write!(formatter, "the actor has no transform setter in its vtable")
            }
            Self::NameHasNul => write!(formatter, "motion name contains a NUL byte"),
        }
    }
}

impl std::error::Error for PuppetError {}
