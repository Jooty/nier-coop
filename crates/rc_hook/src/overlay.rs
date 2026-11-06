mod debug_panel;
mod input;
mod install;
mod levels_panel;
mod mouse_panel;
mod render_loop;
mod style;
mod trainer_panel;

use std::time::Duration;

pub use install::install_when_renderer_ready;

const MIN_GAP_BETWEEN_DRAWN_FRAMES: Duration = Duration::from_millis(2);
const RENDERER_POLL_INTERVAL_MS: u32 = 100;
const RENDERER_POLL_ATTEMPTS: u32 = 300;
const SETTLE_AFTER_RENDERER_LOADED_MS: u32 = 1000;
