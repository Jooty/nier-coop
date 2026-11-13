use crate::banner_art::ArtFocus;
use crate::profile::REPLICANT;

const SEAT_ART_HEIGHT_SHARE: f32 = 0.555;

/// How a lobby seat shows its game's key art until a portrait of the seat's character exists: the point
/// kept at the seat's centre, how much of the art's height the seat shows, and whether it is mirrored.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeatArt {
    pub focus: ArtFocus,
    pub height_share: f32,
    pub mirrored: bool,
}

const CENTRE_SEAT: SeatArt = SeatArt {
    focus: ArtFocus { x: 0.5, y: 0.5 },
    height_share: 1.0,
    mirrored: false,
};
const REPLICANT_SEATS: [SeatArt; 2] = [
    SeatArt {
        focus: ArtFocus { x: 0.503, y: 0.303 },
        height_share: SEAT_ART_HEIGHT_SHARE,
        mirrored: false,
    },
    SeatArt {
        focus: ArtFocus { x: 0.503, y: 0.303 },
        height_share: SEAT_ART_HEIGHT_SHARE,
        mirrored: true,
    },
];

/// The crop of the game's art for seat `seat_index` (0 is the host's seat).
pub fn seat_art(game_id: &str, seat_index: usize) -> SeatArt {
    if game_id != REPLICANT.game_id {
        return CENTRE_SEAT;
    }
    REPLICANT_SEATS[seat_index % REPLICANT_SEATS.len()]
}
