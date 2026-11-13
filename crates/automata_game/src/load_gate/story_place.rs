use serde::{Deserialize, Serialize};

use crate::load_gate::{PlaceRelation, STORY_PLACE_NAME_MOST_BYTES};

/// The phase and sub-phase a game is in: the game's own ids and names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct StoryPlace {
    pub phase_id: i32,
    pub name_len: u8,
    pub name_bytes: [u8; STORY_PLACE_NAME_MOST_BYTES],
}

impl StoryPlace {
    pub fn new(phase_id: i32, sub_phase: &str) -> Self {
        let mut name_bytes = [0_u8; STORY_PLACE_NAME_MOST_BYTES];
        let kept: Vec<u8> = sub_phase
            .bytes()
            .filter(u8::is_ascii_graphic)
            .take(STORY_PLACE_NAME_MOST_BYTES)
            .collect();
        name_bytes[..kept.len()].copy_from_slice(&kept);
        Self {
            phase_id,
            name_len: kept.len() as u8,
            name_bytes,
        }
    }

    pub fn sub_phase(&self) -> String {
        let len = usize::from(self.name_len).min(STORY_PLACE_NAME_MOST_BYTES);
        String::from_utf8_lossy(&self.name_bytes[..len]).into_owned()
    }

    pub fn has_a_name(&self) -> bool {
        self.name_len > 0
    }

    pub fn name_is_valid(&self) -> bool {
        let len = usize::from(self.name_len);
        len <= STORY_PLACE_NAME_MOST_BYTES
            && self.name_bytes[..len].iter().all(u8::is_ascii_graphic)
            && self.name_bytes[len..].iter().all(|byte| *byte == 0)
    }

    pub fn relation_to(&self, other: &Self) -> PlaceRelation {
        if !self.has_a_name() || !other.has_a_name() {
            PlaceRelation::Unknown
        } else if self == other {
            PlaceRelation::Same
        } else {
            PlaceRelation::Other
        }
    }

    pub fn describe(&self) -> String {
        if self.has_a_name() {
            format!("{:#X} {}", self.phase_id, self.sub_phase())
        } else {
            format!("{:#X} (no name)", self.phase_id)
        }
    }
}
