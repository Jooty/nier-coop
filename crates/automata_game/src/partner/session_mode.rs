#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SessionMode {
    /// Exactly two players; the partner is the engine's companion slot body.
    #[default]
    Campaign,
    /// Two to sixteen players and no story rule.
    FreeRoam,
}
