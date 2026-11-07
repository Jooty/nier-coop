mod callback_gate;
mod hooks;
mod scene_follow;
mod start_hold;
mod state;
mod tick;

pub use callback_gate::hold_fired_callback;
pub use hooks::install;
pub use scene_follow::{is_injected_duplicate, note_fired_callback};
pub use state::local_in_cutscene;
pub use tick::tick;
