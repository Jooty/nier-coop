use std::fmt;

use rc_game::build::PeError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateError {
    NoMainModule,
    Header(PeError),
}

impl From<PeError> for GateError {
    fn from(error: PeError) -> Self {
        Self::Header(error)
    }
}

impl fmt::Display for GateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoMainModule => write!(formatter, "the game's main module is not loaded"),
            Self::Header(error) => fmt::Display::fmt(error, formatter),
        }
    }
}

impl std::error::Error for GateError {}
