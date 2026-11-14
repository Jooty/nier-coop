/// Who put the partner body into its flight unit, judged once at each flight start of the partner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlightCase {
    /// The body was not in a unit: the mod boards it.
    MadeByTheMod,
    /// The body was in a unit already: no board call, it is switched to player flight.
    TakenOverFromTheGame,
}

impl FlightCase {
    pub fn label(self) -> &'static str {
        match self {
            Self::MadeByTheMod => "made by the mod",
            Self::TakenOverFromTheGame => "taken over from the game",
        }
    }
}
