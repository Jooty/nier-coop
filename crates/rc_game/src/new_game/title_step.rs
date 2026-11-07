#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleStep {
    AnswerNewGame,
    ShowRealTitle,
}

pub fn title_step(system_data_loaded: bool) -> TitleStep {
    if system_data_loaded {
        TitleStep::AnswerNewGame
    } else {
        TitleStep::ShowRealTitle
    }
}
