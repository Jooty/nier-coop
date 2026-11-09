mod callback_gate;
mod hooks;
mod message_handlers;
mod restart_signals;
mod scene_follow;
mod shared_functions;
mod start_hold;
mod state;
mod tick;

pub use callback_gate::{end_ran_here, hold_fired_callback, latest_partner_end, note_talk_command};
pub(crate) use callback_gate::{park_partner_call, partner_calls_waiting};
pub use hooks::install;
pub(crate) use message_handlers::run_message_handler;
pub(crate) use restart_signals::restart_signals;
pub use scene_follow::{is_injected_duplicate, note_fired_callback};
pub(crate) use shared_functions::{
    close_menus_for_modal_entry, on_function_traced, run_shared_function,
};
pub use state::local_in_cutscene;
pub use tick::tick;
