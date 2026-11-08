mod important_talk_bases;
mod important_talk_ids;
mod press;
mod registry;

pub use important_talk_bases::IMPORTANT_TALK_BASES;
pub use important_talk_ids::IMPORTANT_TALK_IDS;
pub use press::{
    PressAsk, PressVerdict, important_base_of_talk_id, is_important_talk, judge_press,
};
pub use registry::TalkRegistry;

pub const TALK_REGISTRATION_MODE_COUNT: usize = 2;
