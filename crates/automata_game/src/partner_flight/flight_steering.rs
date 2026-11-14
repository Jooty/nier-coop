use crate::partner_flight::{COMPUTER_FLIGHT_STEERING_WORD, PLAYER_FLIGHT_STEERING_WORD};

/// Who steers a rider's flight, by its flight steering word: its controller or the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlightSteering {
    /// Riding, its default next state is free flight 0x70, moved from its controller block.
    Player,
    /// Riding, its default next state is the computer flight state 0x94, which reads no input.
    Computer,
    Other(i32),
}

impl FlightSteering {
    pub fn of_word(word: i32) -> Self {
        match word {
            PLAYER_FLIGHT_STEERING_WORD => Self::Player,
            COMPUTER_FLIGHT_STEERING_WORD => Self::Computer,
            other => Self::Other(other),
        }
    }

    pub fn word(self) -> i32 {
        match self {
            Self::Player => PLAYER_FLIGHT_STEERING_WORD,
            Self::Computer => COMPUTER_FLIGHT_STEERING_WORD,
            Self::Other(word) => word,
        }
    }

    pub fn describe(self) -> String {
        let name = match self {
            Self::Player => "player",
            Self::Computer => "computer",
            Self::Other(_) => "neither",
        };
        format!("{} ({name})", self.word())
    }
}
