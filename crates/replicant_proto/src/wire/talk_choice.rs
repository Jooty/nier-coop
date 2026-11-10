use serde::{Deserialize, Serialize};

use crate::wire::{MAX_DIALOGUE_LINE_INDEX, MAX_TALK_CHOICE_INDEX, MAX_TALK_ID, WireError};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TalkChoiceKind {
    TalkList,
    MenuList,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TalkChoice {
    pub talk_id: u32,
    pub line: u16,
    pub kind: TalkChoiceKind,
    pub index: u8,
    pub target: u32,
}

impl TalkChoice {
    pub fn validate(&self) -> Result<(), WireError> {
        if self.talk_id > MAX_TALK_ID {
            return Err(WireError::OutOfRange { field: "talk_id" });
        }
        if self.line > MAX_DIALOGUE_LINE_INDEX {
            return Err(WireError::OutOfRange { field: "line" });
        }
        if self.index > MAX_TALK_CHOICE_INDEX {
            return Err(WireError::OutOfRange { field: "index" });
        }
        if self.kind == TalkChoiceKind::TalkList && self.target > MAX_TALK_ID {
            return Err(WireError::OutOfRange { field: "target" });
        }
        Ok(())
    }
}
