use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FlightMode {
    #[default]
    None,
    Fighter,
    Robot,
}

impl FlightMode {
    /// The mode a rider's mode word stands for: nonzero is the fighter.
    pub fn of_mode_word(word: i32) -> Self {
        if word != 0 {
            Self::Fighter
        } else {
            Self::Robot
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Fighter => "fighter",
            Self::Robot => "robot",
        }
    }
}
