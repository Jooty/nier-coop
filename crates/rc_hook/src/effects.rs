mod calls;
mod hooks;
mod mirror;
mod report;
mod sound_probe;
mod voice;

pub use hooks::install;
pub(crate) use hooks::{begin_own_hit, end_own_hit};
pub use mirror::tick;

use std::time::Duration;

const PLAYBACK_DELAY: Duration = Duration::from_millis(100);
const MAX_PENDING_EVENTS: usize = 64;
const RETAINED_BLOCK_EXTRA_MS: u64 = 2000;
const SUMMARY_INTERVAL_MS: u64 = 5000;
