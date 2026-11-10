mod rule;
mod state;

pub use rule::{PromptView, may_open_now};
pub use state::{LocalOpen, TutorialPrompts};

pub const OPEN_TUTORIAL_FUNCTION_RVA: u32 = 0x45_F9B0;
pub const TUTORIAL_QUEUE_PROMPT_KIND: i32 = 0x9C5;
