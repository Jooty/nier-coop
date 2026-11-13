use serde::{Deserialize, Serialize};

use crate::load_gate::StoryPlace;

/// The guest asks for the host's done of story action `id`: its own reached done with `value`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoryStepAsk {
    pub id: u32,
    pub value: i32,
    pub place: StoryPlace,
}

impl StoryStepAsk {
    pub fn is_valid(&self) -> bool {
        self.place.name_is_valid()
    }
}
