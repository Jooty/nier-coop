use serde::{Deserialize, Serialize};

use crate::wire::{MAX_STORY_CHANGES_PER_MESSAGE, MAX_STORY_FLAG_ID, STORY_ARRAY_BYTES, WireError};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum StoryArray {
    GameFlags,
    SnowFlags,
    GameCounters,
    SnowCounters,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoryChange {
    Flag {
        array: StoryArray,
        id: u16,
        on: bool,
    },
    Counter {
        array: StoryArray,
        id: u8,
        value: u8,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoryRequest;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoryArrayPart {
    pub array: StoryArray,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoryChanges {
    pub changes: Vec<StoryChange>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoryDigest {
    pub hash: u64,
}

impl StoryArray {
    pub const ALL: [Self; 4] = [
        Self::GameFlags,
        Self::SnowFlags,
        Self::GameCounters,
        Self::SnowCounters,
    ];

    pub fn holds_flags(self) -> bool {
        matches!(self, Self::GameFlags | Self::SnowFlags)
    }
}

impl StoryChange {
    pub fn validate(&self) -> Result<(), WireError> {
        match *self {
            Self::Flag { array, id, .. } => {
                if !array.holds_flags() {
                    return Err(WireError::OutOfRange { field: "array" });
                }
                if id > MAX_STORY_FLAG_ID {
                    return Err(WireError::OutOfRange { field: "id" });
                }
                Ok(())
            }
            Self::Counter { array, .. } if array.holds_flags() => {
                Err(WireError::OutOfRange { field: "array" })
            }
            Self::Counter { .. } => Ok(()),
        }
    }
}

impl StoryArrayPart {
    pub fn validate(&self) -> Result<(), WireError> {
        if self.bytes.len() != STORY_ARRAY_BYTES {
            return Err(WireError::OutOfRange { field: "bytes" });
        }
        Ok(())
    }
}

impl StoryChanges {
    pub fn validate(&self) -> Result<(), WireError> {
        if self.changes.is_empty() || self.changes.len() > MAX_STORY_CHANGES_PER_MESSAGE {
            return Err(WireError::OutOfRange { field: "changes" });
        }
        self.changes.iter().try_for_each(StoryChange::validate)
    }
}
