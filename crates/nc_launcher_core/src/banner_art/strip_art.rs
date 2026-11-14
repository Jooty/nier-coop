use crate::banner_art::ArtFocus;
use crate::profile::{AUTOMATA, REPLICANT};

/// How the free roam board's strip shows its game's key art: the point kept at the strip's centre and how
/// much of the art's height the strip shows. The strip's fade covers its left part, so the subject sits
/// right of the centre in the art.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StripArt {
    pub focus: ArtFocus,
    pub height_share: f32,
}

pub const WHOLE_PICTURE_STRIP: StripArt = StripArt {
    focus: ArtFocus { x: 0.5, y: 0.5 },
    height_share: 1.0,
};
const AUTOMATA_STRIP: StripArt = StripArt {
    focus: ArtFocus { x: 0.47, y: 0.227 },
    height_share: 0.206,
};
const REPLICANT_STRIP: StripArt = StripArt {
    focus: ArtFocus { x: 0.42, y: 0.3 },
    height_share: 0.3,
};

pub fn strip_art(game_id: &str) -> StripArt {
    if game_id == AUTOMATA.game_id {
        AUTOMATA_STRIP
    } else if game_id == REPLICANT.game_id {
        REPLICANT_STRIP
    } else {
        WHOLE_PICTURE_STRIP
    }
}
