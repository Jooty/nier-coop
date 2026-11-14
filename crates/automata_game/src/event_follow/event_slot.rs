use crate::event_follow::{
    EVENT_SLOT_COUNT, EVENT_SLOT_DYING_BIT, EVENT_SLOT_ID_OFFSET, EVENT_SLOT_LOAD_FAILED_STATE,
    EVENT_SLOT_LOADED_STATE, EVENT_SLOT_PLAYING_STATE, EVENT_SLOT_STATE_OFFSET, EVENT_SLOT_STRIDE,
    EVENT_SLOT_USED_BIT, EventId,
};

/// One slot of the game's event player table, as read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventSlot {
    pub flags: u32,
    pub id: EventId,
    pub state: i32,
}

impl EventSlot {
    pub fn of_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < EVENT_SLOT_STATE_OFFSET + 4 {
            return None;
        }
        let word = |at: usize| [bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]];
        Some(Self {
            flags: u32::from_le_bytes(word(0)),
            id: EventId::of_bytes(&bytes[EVENT_SLOT_ID_OFFSET..])?,
            state: i32::from_le_bytes(word(EVENT_SLOT_STATE_OFFSET)),
        })
    }

    pub fn is_live(&self) -> bool {
        self.flags & EVENT_SLOT_USED_BIT != 0 && self.flags & EVENT_SLOT_DYING_BIT == 0
    }

    pub fn is_playing(&self) -> bool {
        self.is_live() && self.state == EVENT_SLOT_PLAYING_STATE
    }

    pub fn is_loaded(&self) -> bool {
        self.is_live() && self.state == EVENT_SLOT_LOADED_STATE
    }

    pub fn load_failed(&self) -> bool {
        self.is_live() && self.state == EVENT_SLOT_LOAD_FAILED_STATE
    }
}

/// The live slots of a table read whole (`EVENT_SLOT_COUNT` times `EVENT_SLOT_STRIDE` bytes).
pub fn slots_of_table(table: &[u8]) -> Vec<EventSlot> {
    table
        .as_chunks::<EVENT_SLOT_STRIDE>()
        .0
        .iter()
        .take(EVENT_SLOT_COUNT)
        .filter_map(|bytes| EventSlot::of_bytes(bytes))
        .filter(EventSlot::is_live)
        .collect()
}
