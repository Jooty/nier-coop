use serde::{Deserialize, Serialize};

use crate::load_gate::StoryPlace;

/// The host tells: its story action `id` reached done with `value`, at `place`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoryStepDone {
    pub number: u32,
    pub id: u32,
    pub value: i32,
    pub place: StoryPlace,
}

impl StoryStepDone {
    pub fn is_valid(&self) -> bool {
        self.number > 0 && self.place.name_is_valid()
    }
}
