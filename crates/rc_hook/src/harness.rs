mod commands;
mod focus;
mod injected_input;
mod key_command;
mod mouse_click;
mod pad;
mod quit;
mod screenshot;
mod settings;
mod state;
mod swap_chain_hook;
mod window_messages;

use std::time::Duration;

pub use commands::reply;
pub use focus::install_window_focus;
pub use injected_input::{InjectedDevice, extend_events, patch_state, virtual_key_is_down};
pub use pad::write_virtual_pad_state;
pub use settings::injection_enabled;
pub use swap_chain_hook::install_screenshot_hook;

const MAX_HOLD: Duration = Duration::from_secs(10);
const DEFAULT_TAP: Duration = Duration::from_millis(100);
const POINTER_SETTLE: Duration = Duration::from_millis(50);
