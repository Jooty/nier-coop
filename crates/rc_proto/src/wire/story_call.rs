use serde::{Deserialize, Serialize};

use crate::wire::WireError;

pub const MAX_STORY_CALL_NAME_BYTES: usize = 63;
pub const MAX_STORY_CALL_ARGS: usize = 6;
pub const MAX_STORY_CALL_TEXT_BYTES: usize = 48;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoryCallArg {
    Number(u64),
    Int(i32),
    Text(Vec<u8>),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoryCall {
    pub phase: u16,
    pub pack: u64,
    pub function: String,
    pub args: Vec<StoryCallArg>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoryRefusalReason {
    Disabled,
    WrongRole,
    NotInPlay,
    WrongPhase,
    NotAStoryCall,
    Duplicate,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoryRefused {
    pub call: StoryCall,
    pub reason: StoryRefusalReason,
}

impl StoryRefused {
    pub fn validate(&self) -> Result<(), WireError> {
        self.call.validate()
    }
}

impl StoryCall {
    pub fn validate(&self) -> Result<(), WireError> {
        let name = self.function.as_bytes();
        if name.is_empty()
            || name.len() > MAX_STORY_CALL_NAME_BYTES
            || !name.iter().all(u8::is_ascii_graphic)
        {
            return Err(WireError::OutOfRange { field: "function" });
        }
        if self.args.len() > MAX_STORY_CALL_ARGS {
            return Err(WireError::OutOfRange { field: "args" });
        }
        let text_too_long = self.args.iter().any(|arg| match arg {
            StoryCallArg::Text(text) => text.len() > MAX_STORY_CALL_TEXT_BYTES || text.contains(&0),
            _ => false,
        });
        if text_too_long {
            return Err(WireError::OutOfRange { field: "text" });
        }
        Ok(())
    }
}
