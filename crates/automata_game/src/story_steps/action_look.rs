use crate::story_steps::{ACTION_DONE_FLAG, ACTION_STARTED_FLAG};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepActionState {
    NotStarted,
    Running,
    Done,
}

/// This copy's story action with an id, as found by the game's lookup (`None`: not loaded here).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionLook {
    pub id: u32,
    pub state: Option<StepActionState>,
    pub value: i32,
}

impl StepActionState {
    pub fn of_flags(flags: u32) -> Self {
        if flags & ACTION_DONE_FLAG != 0 {
            Self::Done
        } else if flags & ACTION_STARTED_FLAG != 0 {
            Self::Running
        } else {
            Self::NotStarted
        }
    }
}
