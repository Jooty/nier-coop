/// What the hook on the game's scene loader says about the end of the load.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HoldReading {
    #[default]
    NotHeld,
    Held,
    Released,
    EndedByTheGame,
    EndedByTheHardLimit,
}

impl HoldReading {
    pub fn is_over(self) -> bool {
        matches!(
            self,
            Self::Released | Self::EndedByTheGame | Self::EndedByTheHardLimit
        )
    }
}
