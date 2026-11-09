use serde::{Deserialize, Serialize};

use crate::wire::{StoryCallArg, WireError, validate_story_function_and_args};

pub const MAX_FIGHT_TAPE_CALL_INDEX: u32 = 65_535;

/// One pop the host's script slot ran inside a fight window, numbered `seq` within `epoch`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FightStreamEntry {
    pub phase: u16,
    pub epoch: u32,
    pub seq: u64,
    pub name: String,
    pub args: Vec<StoryCallArg>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FightWindowChange {
    /// `armed_by`: the arm id whose pop opened the window, streamed as entry 0.
    Opened {
        armed_by: Option<String>,
    },
    /// The host's phase restarted inside the open window: a new epoch of the same window, seq 0.
    Restarted,
    Closed {
        end_seq: u64,
    },
    /// The host's handler of stream entry `seq` ended: every tape value it read was sent before
    /// this mark, so the guest may replay that entry now.
    EntryEnded {
        seq: u64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FightRequestKind {
    /// A pop only the guest's game produced (its player touched a trap or an action target).
    RunPop {
        name: String,
        args: Vec<StoryCallArg>,
    },
    /// The guest's phase restarted and the host's did not follow.
    RestartPhase,
}

/// The host's fight window `number`-`part` of `phase` opened (a new epoch, seq from 0), closed
/// after `end_seq` entries, or one of its entries' handlers ended.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FightWindowMark {
    pub phase: u16,
    pub epoch: u32,
    pub number: u16,
    pub part: u16,
    pub change: FightWindowChange,
}

/// A state-read native's result as the host's game returned it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum FightTapeValue {
    Int(i64),
    Number(f64),
    /// The native returned no value (its actor or group was not there).
    Missing,
}

/// The host's `call_index`-th read of `native` inside the handler of stream entry `seq`; the
/// guest's replay of that entry takes this value instead of its own.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FightTape {
    pub phase: u16,
    pub epoch: u32,
    pub seq: u64,
    pub native: String,
    pub call_index: u32,
    pub value: FightTapeValue,
}

/// The guest asks the host to run something inside the host's fight window `epoch` of `phase`;
/// what the host runs comes back in the stream.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FightRequest {
    pub phase: u16,
    pub epoch: u32,
    pub kind: FightRequestKind,
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
        if let FightWindowChange::Opened {
            armed_by: Some(name),
        } = &self.change
        {
            validate_story_function_and_args(name, &[])?;
        }
        Ok(())
    }
}

impl FightTape {
    pub fn validate(&self) -> Result<(), WireError> {
        validate_story_function_and_args(&self.native, &[])?;
        if self.call_index > MAX_FIGHT_TAPE_CALL_INDEX {
            return Err(WireError::OutOfRange {
                field: "call index",
            });
        }
        if let FightTapeValue::Number(number) = self.value
            && !number.is_finite()
        {
            return Err(WireError::OutOfRange { field: "value" });
        }
        Ok(())
    }
}

impl FightRequest {
    pub fn validate(&self) -> Result<(), WireError> {
        match &self.kind {
            FightRequestKind::RunPop { name, args } => validate_story_function_and_args(name, args),
            FightRequestKind::RestartPhase => Ok(()),
        }
    }
}
