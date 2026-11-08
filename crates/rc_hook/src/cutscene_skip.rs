mod callback_gate;
mod hooks;
mod scene_follow;
mod shared_functions;
mod start_hold;
mod state;
mod tick;

pub(crate) use callback_gate::park_partner_call;
pub use callback_gate::{end_ran_here, hold_fired_callback, latest_partner_end, note_talk_command};
pub use hooks::install;
pub use scene_follow::{is_injected_duplicate, note_fired_callback};
pub(crate) use shared_functions::{on_function_traced, run_shared_function};
pub use state::local_in_cutscene;
pub use tick::tick;
