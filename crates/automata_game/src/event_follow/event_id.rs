use serde::{Deserialize, Serialize};

use crate::event_follow::{EVENT_ID_BYTES, HIGHEST_EVENT_KIND};

/// An event's name in the game: its DataId `{kind, id, sub}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct EventId {
    pub kind: i32,
    pub id: i32,
    pub sub: i32,
}

impl EventId {
    pub fn of_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < EVENT_ID_BYTES {
            return None;
        }
        let word = |at: usize| {
            i32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
        };
        Some(Self {
            kind: word(0),
            id: word(4),
            sub: word(8),
        })
    }

    pub fn to_bytes(self) -> [u8; EVENT_ID_BYTES] {
        let mut bytes = [0_u8; EVENT_ID_BYTES];
        bytes[0..4].copy_from_slice(&self.kind.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.id.to_le_bytes());
        bytes[8..12].copy_from_slice(&self.sub.to_le_bytes());
        bytes
    }

    pub fn is_valid(&self) -> bool {
        (0..=HIGHEST_EVENT_KIND).contains(&self.kind)
    }

    pub fn describe(&self) -> String {
        format!(
            "event kind {} id {:#X} sub {}",
            self.kind, self.id, self.sub
        )
    }
}
