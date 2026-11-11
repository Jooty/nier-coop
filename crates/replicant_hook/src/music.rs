mod calls;
mod hooks;
mod report;
mod tick;

pub use hooks::install;
pub use tick::tick;

use std::time::Duration;

const LOG_LINES_PER_WINDOW: u32 = 20;
const LOG_WINDOW_MS: u64 = 5000;
const DIAGNOSTIC_INTERVAL: Duration = Duration::from_secs(10);
const PLAY_FADE_SECONDS: f32 = 0.0;
const VOLUME_RAMP_SECONDS: f32 = 0.0;
