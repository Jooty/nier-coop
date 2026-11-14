use crate::load_gate::{FIRST_SYSTEM_PHASE_ID, StoryPlace};
use crate::story_follow::StoryRequest;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestClass {
    System,
    SamePlace,
    Story,
}

impl RequestClass {
    pub fn of(request: &StoryRequest, current: &StoryPlace) -> Self {
        let target = request.target_place();
        if request.id() < 0 || request.id() >= FIRST_SYSTEM_PHASE_ID {
            Self::System
        } else if target.has_a_name() && target == *current {
            Self::SamePlace
        } else {
            Self::Story
        }
    }

    pub fn is_for_the_story(self) -> bool {
        self == Self::Story
    }
}
