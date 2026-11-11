use crate::net::MAX_LOBBY_SLOT;
use crate::saves::SlotSummary;

pub fn new_game_slot(slots: &[SlotSummary]) -> Option<u8> {
    (0..=MAX_LOBBY_SLOT).find(|index| !slots.iter().any(|slot| slot.index == *index && slot.used))
}
