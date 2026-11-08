mod important_talk_bases;
mod press;
mod registry;

pub use important_talk_bases::IMPORTANT_TALK_BASES;
pub use press::{PressAsk, PressVerdict, is_important_talk, judge_press};
pub use registry::TalkRegistry;

pub const TALK_REGISTRATION_MODE_COUNT: usize = 2;
