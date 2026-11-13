use serde::{Deserialize, Serialize};

use crate::story_follow::AskVerdict;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoryAskAnswer {
    pub number: u32,
    pub verdict: AskVerdict,
}
