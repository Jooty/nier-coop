use serde::{Deserialize, Serialize};

/// Why the host has no done to give for an ask (a done is told with `StoryStepDone` instead).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepAnswer {
    NotStarted,
    OtherPlace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoryStepAnswer {
    pub id: u32,
    pub answer: StepAnswer,
}

impl StepAnswer {
    pub fn label(self) -> &'static str {
        match self {
            Self::NotStarted => "the host's action has not started",
            Self::OtherPlace => "the host is at another place",
        }
    }
}
