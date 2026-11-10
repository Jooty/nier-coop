use std::fmt;

use crate::memory::MemoryError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameError {
    Memory(MemoryError),
    PlayableActorNotSpawned,
    UnknownItem(String),
    TextTooLong { len: usize, capacity: usize },
    UnexpectedBytes { rva: u32 },
}

impl From<MemoryError> for GameError {
    fn from(error: MemoryError) -> Self {
        Self::Memory(error)
    }
}

impl fmt::Display for GameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Memory(error) => fmt::Display::fmt(error, formatter),
            Self::PlayableActorNotSpawned => {
                write!(formatter, "the playable actor pointer is null")
            }
            Self::UnknownItem(name) => write!(formatter, "unknown inventory item {name:?}"),
            Self::TextTooLong { len, capacity } => {
                write!(
                    formatter,
                    "text of {len} bytes does not fit in {capacity} bytes"
                )
            }
            Self::UnexpectedBytes { rva } => {
                write!(
                    formatter,
                    "code at rva {rva:#x} is neither original nor patched"
                )
            }
        }
    }
}

impl std::error::Error for GameError {}
