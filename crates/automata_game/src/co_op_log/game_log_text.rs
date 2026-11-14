use crate::co_op_log::{FIRST_GAME_LOG_TEXT_ID, GAME_LOG_TEXT_MAX_UNITS, GAME_LOG_TEXT_SLOT_COUNT};
use crate::partner_hud::CHARACTERS_THE_GAME_TEXT_MAY_READ_AS_MARKUP;

/// The text id the mod gives the game's log for one of its own lines.
pub fn game_log_text_id(slot: usize) -> u32 {
    FIRST_GAME_LOG_TEXT_ID + (slot % GAME_LOG_TEXT_SLOT_COUNT) as u32
}

/// The mod's line a text id stands for; `None` for every id of the game's own text.
pub fn game_log_text_slot(text_id: u32) -> Option<usize> {
    let slot = text_id.checked_sub(FIRST_GAME_LOG_TEXT_ID)? as usize;
    (slot < GAME_LOG_TEXT_SLOT_COUNT).then_some(slot)
}

/// A line as the game's text calls take it: UTF-16 with a zero at the end, bounded, without
/// control characters or anything the game's text might read as markup.
pub fn game_log_text_units(line: &str) -> Vec<u16> {
    let kept: String = line
        .chars()
        .filter(|character| {
            !character.is_control()
                && !CHARACTERS_THE_GAME_TEXT_MAY_READ_AS_MARKUP.contains(character)
        })
        .collect();
    let mut units: Vec<u16> = kept.encode_utf16().take(GAME_LOG_TEXT_MAX_UNITS).collect();
    // A cut must not leave half of a two-unit character at the end.
    if units
        .last()
        .is_some_and(|last| (0xD800..0xDC00).contains(last))
    {
        units.pop();
    }
    units.push(0);
    units
}
