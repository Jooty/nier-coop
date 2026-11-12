use crate::banner_art::ArtFocus;
use crate::profile::{AUTOMATA, REPLICANT};

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
const AUTOMATA_SEATS: [SeatArt; 2] = [
    SeatArt {
        focus: ArtFocus { x: 0.566, y: 0.387 },
        height_share: SEAT_ART_HEIGHT_SHARE,
        mirrored: false,
    },
    SeatArt {
        focus: ArtFocus { x: 0.433, y: 0.609 },
        height_share: SEAT_ART_HEIGHT_SHARE,
        mirrored: false,
    },
];
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

/// The crop of the game's art for seat `seat_index` (0 is the host's seat). Seats past the second reuse the
/// first two in turn.
pub fn seat_art(game_id: &str, seat_index: usize) -> SeatArt {
    let seats: &[SeatArt] = if game_id == REPLICANT.game_id {
        &REPLICANT_SEATS
    } else if game_id == AUTOMATA.game_id {
        &AUTOMATA_SEATS
    } else {
        return CENTRE_SEAT;
    };
    seats[seat_index % seats.len()]
}
