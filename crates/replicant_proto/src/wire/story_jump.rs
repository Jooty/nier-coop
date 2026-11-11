use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoryJumpPoint {
    AfterPrologue,
    AtPopolaTalk,
    BeforeNextDay,
    AtShrineEntrance,
    AtShrineTop,
    AtShrineBoss,
    AerieEntrance,
    JunkHeapAfterFirstUpgrade,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoryJump {
    pub point: StoryJumpPoint,
}
