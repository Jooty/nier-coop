use serde::{Deserialize, Serialize};

use crate::wire::{MAX_DIALOGUE_LINE_INDEX, MAX_TALK_ID, WireError};

/// The talk window line a player is reading: the game's talk id and the line index inside it.
/// Each game looks the text up in its own talk data, so no dialogue text crosses the wire.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogueLine {
    pub talk_id: u32,
    pub line: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DialogueUpdate {
    Line(DialogueLine),
    Ended,
}

impl DialogueUpdate {
    pub fn validate(&self) -> Result<(), WireError> {
        match self {
            Self::Line(line) if line.talk_id > MAX_TALK_ID => {
                Err(WireError::OutOfRange { field: "talk_id" })
            }
            Self::Line(line) if line.line > MAX_DIALOGUE_LINE_INDEX => {
                Err(WireError::OutOfRange { field: "line" })
            }
            Self::Line(_) | Self::Ended => Ok(()),
        }
    }
}
