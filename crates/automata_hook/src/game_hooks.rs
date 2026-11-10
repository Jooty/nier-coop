mod camera_guard;
mod input_block;
mod input_feed;
mod partner_walk_guards;
mod save_redirect;
mod second_copy;
mod start_recorder;
mod story_gate;

pub use camera_guard::install_camera_guard;
pub use input_block::{install_input_block, set_menu_open};
pub use input_feed::{FeedHooks, clear_input_feed, install_input_feed, set_input_feed};
pub use partner_walk_guards::{guard_held_body, install_partner_walk_guards};
pub use save_redirect::{coop_save_file_exists, install_save_redirect, save_redirect_status};
pub use second_copy::{install_second_copy_guard, log_second_copy_state};
pub use start_recorder::{forget_recorded_start, install_start_recorder, recorded_start};
pub use story_gate::{
    arm_story_gate, disarm_story_gate, expect_travel_scene_request, install_story_gate,
    run_as_ours, story_gate_status,
};
