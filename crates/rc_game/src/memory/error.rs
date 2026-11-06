use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryError {
    Unreadable { address: u64 },
    Unwritable { address: u64 },
}

impl fmt::Display for MemoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable { address } => write!(formatter, "cannot read memory at {address:#x}"),
            Self::Unwritable { address } => {
                write!(formatter, "cannot write memory at {address:#x}")
            }
        }
    }
}

impl std::error::Error for MemoryError {}
