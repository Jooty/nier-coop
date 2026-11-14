use crate::banner_art::{ArtFocus, BannerFacts};
use crate::launcher_game::{GameSaves, LauncherGame};
use crate::profile::REPLICANT;
use crate::profile::replicant::replicant_saves::REPLICANT_SAVES;

const REPLICANT_BANNER_TAGS: [&str; 2] = ["Co-op campaign", "2 players"];
const REPLICANT_ART_FOCUS: ArtFocus = ArtFocus { x: 0.51, y: 0.45 };

#[derive(Debug)]
pub struct ReplicantLauncherGame;

pub const REPLICANT_LAUNCHER_GAME: ReplicantLauncherGame = ReplicantLauncherGame;

impl LauncherGame for ReplicantLauncherGame {
    fn game_id(&self) -> &'static str {
        REPLICANT.game_id
    }

    fn banner(&self) -> BannerFacts {
        BannerFacts {
            tags: &REPLICANT_BANNER_TAGS,
            focus: REPLICANT_ART_FOCUS,
        }
    }

    fn serif_title(&self) -> bool {
        true
    }

    fn game_dir_in_legacy_config_field(&self) -> bool {
        true
    }

    fn saves(&self) -> Option<&'static dyn GameSaves> {
        Some(&REPLICANT_SAVES)
    }
}
