mod hooks;
mod scene_cover;
mod tick;

pub use hooks::{arm_host_zone_change, install};
pub use scene_cover::{note_scene_follow_state, step_scene_covers};
pub(crate) use tick::{fade_guest_back_in, fade_on_this_game_only};
pub use tick::{guest_holds_fade, tick};
