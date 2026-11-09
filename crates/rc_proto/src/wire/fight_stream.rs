use serde::{Deserialize, Serialize};

use crate::wire::{StoryCallArg, WireError, validate_story_function_and_args};

/// One pop the host's script slot ran inside a fight window, numbered `seq` within `epoch`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FightStreamEntry {
    pub phase: u16,
    pub epoch: u32,
    pub seq: u64,
    pub name: String,
    pub args: Vec<StoryCallArg>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FightWindowChange {
    Opened,
    Closed { end_seq: u64 },
}

/// The host's fight window `number`-`part` of `phase` opened (a new epoch, seq from 0) or closed
/// after `end_seq` entries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FightWindowMark {
    pub phase: u16,
    pub epoch: u32,
    pub number: u16,
    pub part: u16,
    pub change: FightWindowChange,
}

impl FightStreamEntry {
    pub fn validate(&self) -> Result<(), WireError> {
        validate_story_function_and_args(&self.name, &self.args)
    }
}

impl FightWindowMark {
    pub fn validate(&self) -> Result<(), WireError> {
        if self.number == 0 || self.part == 0 {
            return Err(WireError::OutOfRange {
                field: "fight window",
            });
        }
        Ok(())
    }
}
