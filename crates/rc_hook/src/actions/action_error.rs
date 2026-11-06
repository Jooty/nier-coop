use std::fmt;

use rc_game::offsets::MAX_PHASE_POINT;

use crate::actions::MAX_GOLD_PER_ACTION;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionError {
    UnknownPhase(String),
    PointOutOfRange { point: i32 },
    GoldOutOfRange { amount: i32 },
}

impl fmt::Display for ActionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownPhase(name) => write!(formatter, "unknown phase {name:?}"),
            Self::GoldOutOfRange { amount } => {
                write!(
                    formatter,
                    "gold {amount} is outside 1 to {MAX_GOLD_PER_ACTION}"
                )
            }
            Self::PointOutOfRange { point } => {
                write!(formatter, "point {point} is outside 0 to {MAX_PHASE_POINT}")
            }
        }
    }
}

impl std::error::Error for ActionError {}
