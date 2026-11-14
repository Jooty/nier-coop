use serde::{Deserialize, Serialize};

use crate::load_gate::StoryPlace;
use crate::story_follow::StoryRequest;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoryAsk {
    pub number: u32,
    pub at: StoryPlace,
    pub request: StoryRequest,
}

impl StoryAsk {
    pub fn is_valid(&self) -> bool {
        self.number > 0 && self.at.name_is_valid() && self.request.is_valid()
    }
}
