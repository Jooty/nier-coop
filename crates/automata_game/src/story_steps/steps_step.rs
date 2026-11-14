use crate::story_steps::{StoryStepAnswer, StoryStepAsk, StoryStepDone};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepSend {
    Done(StoryStepDone),
    Ask(StoryStepAsk),
    Answer(StoryStepAnswer),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetDoneWhy {
    HostsDone,
    AskedByTheGuest,
    GaveUpWaiting,
    LinkLost,
}

/// Set story action `id` done in this copy's game, only while it runs (started, not done).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetDone {
    pub id: u32,
    pub value: i32,
    pub why: SetDoneWhy,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StepsStep {
    pub sends: Vec<StepSend>,
    pub set_done: Vec<SetDone>,
    pub logs: Vec<String>,
}

impl SetDoneWhy {
    pub fn label(self) -> &'static str {
        match self {
            Self::HostsDone => "the host's done",
            Self::AskedByTheGuest => "the guest asked and this action runs here",
            Self::GaveUpWaiting => "no word from the host in time, this game's own done passes",
            Self::LinkLost => "the link is lost, this game's own done passes",
        }
    }
}
