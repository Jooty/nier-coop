mod click_sources;
mod co_op_panel;
mod debug_panel;
mod enemy_sync_section;
mod hud_assets;
mod input;
mod install;
mod levels_panel;
mod mouse_panel;
mod partner_marker;
mod party_frame;
mod render_loop;
mod stay_together_panel;
mod style;
mod swapchain_window;
mod trainer_panel;
mod zone_prompt;

pub use install::install_when_renderer_ready;

const RENDERER_POLL_INTERVAL_MS: u32 = 100;
const RENDERER_POLL_ATTEMPTS: u32 = 300;
const SETTLE_AFTER_RENDERER_LOADED_MS: u32 = 1000;
