mod autopilot_setting;
mod circle;

pub use autopilot_setting::Autopilot;
pub use circle::{StickFeed, autopilot_feed};

pub const AUTOPILOT_CIRCLE_PERIOD_MS: u64 = 20_000;
pub const AUTOPILOT_STICK_LENGTH: f32 = 600.0;
