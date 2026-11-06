mod click_sources;
mod co_op_panel;
mod debug_panel;
mod input;
mod install;
mod levels_panel;
mod mouse_panel;
mod render_loop;
mod style;
mod swapchain_window;
mod trainer_panel;

pub use install::install_when_renderer_ready;

const RENDERER_POLL_INTERVAL_MS: u32 = 100;
const RENDERER_POLL_ATTEMPTS: u32 = 300;
const SETTLE_AFTER_RENDERER_LOADED_MS: u32 = 1000;
