use serde::{Deserialize, Serialize};

/// What kind of flight a hero is in, by the game's own flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FlightKind {
    #[default]
    NotInAUnit,
    /// The riding flag is set and the shooting stage flag is not.
    FreeFlight,
    /// The shooting stage flag is set; the game leaves the riding flag clear in a stage.
    ShootingStage,
}

impl FlightKind {
    pub fn is_in_a_unit(self) -> bool {
        self != Self::NotInAUnit
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::NotInAUnit => "not in a unit",
            Self::FreeFlight => "free flight",
            Self::ShootingStage => "shooting stage",
        }
    }
}
