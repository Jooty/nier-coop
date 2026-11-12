use crate::profile::{AUTOMATA, REPLICANT};

const REPLICANT_BANNER_TAGS: [&str; 2] = ["Co-op campaign", "2 players"];
const AUTOMATA_BANNER_TAGS: [&str; 3] = ["Co-op campaign", "Free roam", "2 to 16 players"];
const REPLICANT_TITLE_NOTE: &str = "ver.1.22474487139";

/// A point in the art, as fractions of its width and height, that a crop keeps in frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArtFocus {
    pub x: f32,
    pub y: f32,
}

/// What a game's home banner shows besides the install facts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BannerFacts {
    pub tags: &'static [&'static str],
    pub title_note: Option<&'static str>,
    pub focus: ArtFocus,
}

const CENTRE_FOCUS: ArtFocus = ArtFocus { x: 0.5, y: 0.5 };
const REPLICANT_ART_FOCUS: ArtFocus = ArtFocus { x: 0.51, y: 0.45 };
const AUTOMATA_ART_FOCUS: ArtFocus = ArtFocus { x: 0.56, y: 0.4 };

pub fn banner_facts(game_id: &str) -> BannerFacts {
    if game_id == REPLICANT.game_id {
        BannerFacts {
            tags: &REPLICANT_BANNER_TAGS,
            title_note: Some(REPLICANT_TITLE_NOTE),
            focus: REPLICANT_ART_FOCUS,
        }
    } else if game_id == AUTOMATA.game_id {
        BannerFacts {
            tags: &AUTOMATA_BANNER_TAGS,
            title_note: None,
            focus: AUTOMATA_ART_FOCUS,
        }
    } else {
        BannerFacts {
            tags: &[],
            title_note: None,
            focus: CENTRE_FOCUS,
        }
    }
}
