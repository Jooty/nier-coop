#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenGuess {
    InGame,
    Loading,
    TitleOrMenu,
}

impl ScreenGuess {
    pub fn name(self) -> &'static str {
        match self {
            Self::InGame => "in_game",
            Self::Loading => "loading",
            Self::TitleOrMenu => "title_or_menu",
        }
    }
}

pub fn guess_screen(player_present: bool, map_manager_active: bool) -> ScreenGuess {
    match (player_present, map_manager_active) {
        (true, true) => ScreenGuess::InGame,
        (true, false) | (false, true) => ScreenGuess::Loading,
        (false, false) => ScreenGuess::TitleOrMenu,
    }
}
