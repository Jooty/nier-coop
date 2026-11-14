use serde::{Deserialize, Serialize};

use crate::load_gate::StoryPlace;
use crate::story_follow::StoryRequest;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoryMove {
    pub number: u32,
    pub from: StoryPlace,
    pub request: StoryRequest,
}

impl StoryMove {
    pub fn is_valid(&self) -> bool {
        self.number > 0 && self.from.name_is_valid() && self.request.is_valid()
    }
}
