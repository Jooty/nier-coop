use crate::story_follow::{StoryAsk, StoryAskAnswer, StoryMove, StoryRequest};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FollowSend {
    Move(StoryMove),
    Ask(StoryAsk),
    Answer(StoryAskAnswer),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FollowStep {
    pub sends: Vec<FollowSend>,
    pub make: Option<StoryRequest>,
    pub logs: Vec<String>,
}
