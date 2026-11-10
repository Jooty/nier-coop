mod input_block;
mod save_redirect;
mod second_copy;
mod story_gate;

pub use input_block::{install_input_block, set_menu_open};
pub use save_redirect::{coop_save_folder, install_save_redirect, save_redirect_status};
pub use second_copy::{install_second_copy_guard, log_second_copy_state};
pub use story_gate::{arm_story_gate, install_story_gate, run_as_ours};
