use std::time::SystemTime;

use crate::start::HIGHEST_SAVE_SLOT;

/// The save slot whose file was written last; a tie goes to the lower slot.
pub fn newest_slot(slot_files: &[(u32, SystemTime)]) -> Option<u32> {
    slot_files
        .iter()
        .filter(|(slot, _)| *slot <= HIGHEST_SAVE_SLOT)
        .max_by(|(slot_a, written_a), (slot_b, written_b)| {
            written_a.cmp(written_b).then(slot_b.cmp(slot_a))
        })
        .map(|(slot, _)| *slot)
}
