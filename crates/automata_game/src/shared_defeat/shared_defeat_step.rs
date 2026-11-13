use crate::shared_defeat::{DefeatNote, GameOverMenuWrite};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SharedDefeatStep {
    pub tell: Vec<DefeatNote>,
    pub menu_write: Option<GameOverMenuWrite>,
    pub logs: Vec<String>,
}
