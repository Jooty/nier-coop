use serde::{Deserialize, Serialize};

use crate::shared_defeat::{
    GAME_OVER_MENU_CHOOSING_STATE, GAME_OVER_MENU_CHOOSING_SUB_STATE,
    GAME_OVER_MENU_FADE_OUT_STATE, GAME_OVER_MENU_RETRY_CHOSEN_STATE,
    GAME_OVER_MENU_SECOND_LIST_STATE, GAME_OVER_MENU_TITLE_CURSOR,
};

/// The words of the game's own game-over menu object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GameOverMenu {
    pub done: i32,
    pub state: i32,
    pub sub_state: i32,
    pub cursor: i32,
}

/// The two choices the game-over menu offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameOverChoice {
    Retry,
    Title,
}

impl GameOverChoice {
    pub fn label(self) -> &'static str {
        match self {
            Self::Retry => "retry",
            Self::Title => "title",
        }
    }
}

impl GameOverMenu {
    /// The menu shows its two items and takes a confirm (state 3, sub-state 2).
    pub fn is_choosing(self) -> bool {
        self.state == GAME_OVER_MENU_CHOOSING_STATE
            && self.sub_state == GAME_OVER_MENU_CHOOSING_SUB_STATE
    }

    /// The choice the menu's own choose step made between the reading `before` and this one.
    pub fn choice_since(self, before: Self) -> Option<GameOverChoice> {
        if before.state != GAME_OVER_MENU_CHOOSING_STATE {
            return None;
        }
        match self.state {
            GAME_OVER_MENU_RETRY_CHOSEN_STATE | GAME_OVER_MENU_SECOND_LIST_STATE => {
                Some(GameOverChoice::Retry)
            }
            GAME_OVER_MENU_FADE_OUT_STATE if self.cursor == GAME_OVER_MENU_TITLE_CURSOR => {
                Some(GameOverChoice::Title)
            }
            _ => None,
        }
    }

    pub fn describe(self) -> String {
        format!(
            "menu state {}, sub-state {}, cursor {}, done {}",
            self.state, self.sub_state, self.cursor, self.done
        )
    }
}
