use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CutsceneSkipAsk;

/// A player's answer to a game prompt whose answer skips a sequence both players are in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkipPromptAnswer {
    pub phase: u16,
    pub prompt: u16,
    pub skip: bool,
}

/// The host's end of a skip vote: which answer ran there (`skip` false is the other answer, or no
/// answer for a skip with none). The guest runs the same answer, so the sequence ends on both.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkipPromptVerdict {
    pub phase: u16,
    pub prompt: u16,
    pub skip: bool,
}
