use serde::{Deserialize, Serialize};

use crate::load_gate::{PlaceRelation, STORY_PLACE_NAME_MOST_BYTES};

/// The phase and sub-phase a game is in: the game's own ids and names.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct StoryPlace {
    pub phase_id: i32,
    pub sub_phase: String,
}

impl StoryPlace {
    pub fn new(phase_id: i32, sub_phase: &str) -> Self {
        let mut end = sub_phase.len().min(STORY_PLACE_NAME_MOST_BYTES);
        while !sub_phase.is_char_boundary(end) {
            end -= 1;
        }
        Self {
            phase_id,
            sub_phase: sub_phase[..end].to_owned(),
        }
    }

    pub fn has_a_name(&self) -> bool {
        !self.sub_phase.is_empty()
    }

    pub fn name_is_valid(&self) -> bool {
        self.sub_phase.len() <= STORY_PLACE_NAME_MOST_BYTES
            && self.sub_phase.bytes().all(|byte| byte.is_ascii_graphic())
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
            format!("{:#X} {}", self.phase_id, self.sub_phase)
        } else {
            format!("{:#X} (no name)", self.phase_id)
        }
    }
}
