use rc_game::story_progress::fix_header_checksum;

use crate::saves::coop::blank_file_from;
use crate::saves::{
    GAMEDATA_SIZE, HEADER_ENDING_RECORD_RANGES, HEADER_SLOT_MASK_OFFSET, SlotError,
};

pub fn fresh_coop_file(vanilla: Option<&[u8]>) -> Result<Vec<u8>, SlotError> {
    let vanilla = vanilla.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no normal save file to copy the header from; the game creates it",
        )
    })?;
    if vanilla.len() != GAMEDATA_SIZE {
        return Err(SlotError::WrongSize(vanilla.len()));
    }
    let mut fresh = blank_file_from(vanilla);
    fresh[HEADER_SLOT_MASK_OFFSET] = 0;
    for range in HEADER_ENDING_RECORD_RANGES {
        fresh[range].fill(0);
    }
    fix_header_checksum(&mut fresh).map_err(|_| SlotError::WrongSize(fresh.len()))?;
    Ok(fresh)
}
