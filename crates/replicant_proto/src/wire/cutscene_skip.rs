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
