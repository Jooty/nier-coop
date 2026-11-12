use nc_launcher_core::{AUTOMATA, GameProfile, REPLICANT};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMark {
    Circle,
    Diamond,
}

/// How a game's name is set: the first game in EB Garamond, every other one in light spaced capitals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleFace {
    Garamond,
    SpacedCapitals,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameLook {
    pub mark: GameMark,
    pub title_face: TitleFace,
}

pub fn game_look(profile: &GameProfile) -> GameLook {
    if profile.game_id == AUTOMATA.game_id {
        GameLook {
            mark: GameMark::Diamond,
            title_face: TitleFace::SpacedCapitals,
        }
    } else if profile.game_id == REPLICANT.game_id {
        GameLook {
            mark: GameMark::Circle,
            title_face: TitleFace::Garamond,
        }
    } else {
        neutral_look()
    }
}

pub fn neutral_look() -> GameLook {
    GameLook {
        mark: GameMark::Circle,
        title_face: TitleFace::SpacedCapitals,
    }
}
