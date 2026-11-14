use std::fmt::Debug;

use crate::banner_art::{ArtFocus, BannerFacts};
use crate::launcher_game::GameSaves;

const NEUTRAL_ART_FOCUS: ArtFocus = ArtFocus { x: 0.5, y: 0.5 };

pub trait LauncherGame: Debug + Sync {
    fn game_id(&self) -> &'static str;

    fn banner(&self) -> BannerFacts {
        BannerFacts {
            tags: &[],
            focus: NEUTRAL_ART_FOCUS,
        }
    }

    fn serif_title(&self) -> bool {
        false
    }

    fn game_dir_in_legacy_config_field(&self) -> bool {
        false
    }

    fn saves(&self) -> Option<&'static dyn GameSaves> {
        None
    }
}
