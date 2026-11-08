mod action;
mod action_error;
mod phase_change;
mod queue;
mod runner;
mod tick_hook;
mod trainer;

pub use action::Action;
pub use action_error::ActionError;
pub(crate) use phase_change::change_phase;
pub use queue::enqueue;
pub use tick_hook::{install, is_draining};

const MAX_QUEUED_ACTIONS: usize = 16;
const MAX_GOLD_PER_ACTION: i32 = 100_000;
