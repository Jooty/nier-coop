use eframe::egui::Color32;
use nc_launcher_core::{AUTOMATA, GameProfile, REPLICANT};

use crate::app::palette;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMark {
    Circle,
    Diamond,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameLook {
    pub accent: Color32,
    pub mark: GameMark,
}

impl GameLook {
    pub fn accent_soft(self) -> Color32 {
        let [red, green, blue, _] = self.accent.to_array();
        Color32::from_rgba_unmultiplied(red, green, blue, palette::ACCENT_SOFT_ALPHA)
    }
}

pub fn game_look(profile: &GameProfile) -> GameLook {
    if profile.game_id == AUTOMATA.game_id {
        GameLook {
            accent: palette::AUTOMATA_ACCENT,
            mark: GameMark::Diamond,
        }
    } else if profile.game_id == REPLICANT.game_id {
        GameLook {
            accent: palette::REPLICANT_ACCENT,
            mark: GameMark::Circle,
        }
    } else {
        neutral_look()
    }
}

pub fn neutral_look() -> GameLook {
    GameLook {
        accent: palette::ACCENT,
        mark: GameMark::Circle,
    }
}
