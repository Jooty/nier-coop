mod callback_gate;
mod hooks;
mod scene_follow;
mod start_hold;
mod state;
mod tick;
mod view_follow;

pub use callback_gate::{end_ran_here, hold_fired_callback, latest_partner_end, note_talk_command};
pub use hooks::install;
pub use scene_follow::{is_injected_duplicate, note_fired_callback};
pub use state::local_in_cutscene;
pub use tick::tick;
pub use view_follow::{note_door_callback, note_scene_run};
