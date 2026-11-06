use std::fmt;

use minhook::MH_STATUS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetourError {
    RvaOutsideImage { rva: u32 },
    MinHook(MH_STATUS),
}

impl From<MH_STATUS> for DetourError {
    fn from(status: MH_STATUS) -> Self {
        Self::MinHook(status)
    }
}

impl fmt::Display for DetourError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RvaOutsideImage { rva } => {
                write!(formatter, "rva {rva:#x} is outside the game image")
            }
            Self::MinHook(status) => write!(formatter, "minhook failed: {status}"),
        }
    }
}

impl std::error::Error for DetourError {}
