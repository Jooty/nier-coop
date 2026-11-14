use crate::banner_art::{ArtFocus, BannerFacts, StripArt};
use crate::launcher_game::{GameLink, LauncherGame};
use crate::profile::AUTOMATA;
use crate::profile::automata::automata_link::AUTOMATA_LINK;

const AUTOMATA_BANNER_TAGS: [&str; 3] = ["Co-op campaign", "Free roam", "2 to 16 players"];
const AUTOMATA_ART_FOCUS: ArtFocus = ArtFocus { x: 0.56, y: 0.4 };
const AUTOMATA_STRIP: StripArt = StripArt {
    focus: ArtFocus { x: 0.47, y: 0.227 },
    height_share: 0.206,
};
const AUTOMATA_WHOLE_COOP_SAVES_FOLDER: &str = "NieR_Automata_coop";

#[derive(Debug)]
pub struct AutomataLauncherGame;

pub const AUTOMATA_LAUNCHER_GAME: AutomataLauncherGame = AutomataLauncherGame;

impl LauncherGame for AutomataLauncherGame {
    fn game_id(&self) -> &'static str {
        AUTOMATA.game_id
    }

    fn link(&self) -> &'static dyn GameLink {
        &AUTOMATA_LINK
    }

    fn banner(&self) -> BannerFacts {
        BannerFacts {
            tags: &AUTOMATA_BANNER_TAGS,
            focus: AUTOMATA_ART_FOCUS,
        }
    }

    fn diamond_mark(&self) -> bool {
        true
    }

    fn strip_art(&self) -> StripArt {
        AUTOMATA_STRIP
    }

    fn whole_coop_saves_folder(&self) -> Option<&'static str> {
        Some(AUTOMATA_WHOLE_COOP_SAVES_FOLDER)
    }
}
