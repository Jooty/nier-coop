/// What the hook writes into this copy's game-over menu to make the other copy's choice here,
/// each exactly what the menu's own choose step writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameOverMenuWrite {
    /// One phase request for the current phase and sub-phase with the retry parameters, then
    /// cursor 0 (the flow reads it when the menu is done) and menu state 6. Only while the
    /// in-place retry bit is set.
    RetryInPlace,
    /// Cursor 1, sub-state 2, menu state 5.
    Title,
}

impl GameOverMenuWrite {
    pub fn label(self) -> &'static str {
        match self {
            Self::RetryInPlace => "retry in place",
            Self::Title => "title",
        }
    }
}
