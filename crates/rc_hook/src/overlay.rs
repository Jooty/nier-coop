mod click_sources;
mod co_op_panel;
mod debug_panel;
mod input;
mod install;
mod levels_panel;
mod mouse_panel;
mod puppet_panel;
mod render_loop;
mod style;
mod trainer_panel;

use std::time::Duration;

pub use input::game_window;
pub use install::install_when_renderer_ready;
pub use mouse_panel::latest_mouse_json;

const MIN_GAP_BETWEEN_DRAWN_FRAMES: Duration = Duration::from_millis(2);
const RENDERER_POLL_INTERVAL_MS: u32 = 100;
const RENDERER_POLL_ATTEMPTS: u32 = 300;
const SETTLE_AFTER_RENDERER_LOADED_MS: u32 = 1000;
