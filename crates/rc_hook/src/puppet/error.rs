use std::fmt;

use rc_game::GameError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PuppetError {
    Game(GameError),
    CreateReturnedNull,
    UnknownPuppet(u32),
    TransformSetterMissing,
    VirtualFunctionMissing(u64),
    UnknownAttachPoint(i32),
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
            Self::UnknownPuppet(id) => write!(formatter, "no puppet with id {id}"),
            Self::TransformSetterMissing => {
                write!(formatter, "the actor has no transform setter in its vtable")
            }
            Self::VirtualFunctionMissing(offset) => {
                write!(
                    formatter,
                    "the actor has no function at vtable offset {offset:#x}"
                )
            }
            Self::UnknownAttachPoint(id) => {
                write!(formatter, "the player has no attach point {id}")
            }
            Self::NameHasNul => write!(formatter, "a name contains a NUL byte"),
        }
    }
}

impl std::error::Error for PuppetError {}
