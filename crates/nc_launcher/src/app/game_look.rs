use nc_launcher_core::{GameProfile, game_of};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMark {
    Circle,
    Diamond,
}

/// How a game's name is set: in EB Garamond when its profile asks for a serif title, else in light spaced capitals.
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
    let game = game_of(profile);
    if game.serif_title() {
        GameLook {
            mark: GameMark::Circle,
            title_face: TitleFace::Garamond,
        }
    } else if game.diamond_mark() {
        GameLook {
            mark: GameMark::Diamond,
            title_face: TitleFace::SpacedCapitals,
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
