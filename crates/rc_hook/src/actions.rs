mod action;
mod action_error;
mod phase_change;
mod queue;
mod runner;
mod tick_hook;
mod trainer;

pub use action::Action;
pub use action_error::ActionError;
pub use queue::enqueue;
pub use tick_hook::{install, is_draining};

const MAX_QUEUED_ACTIONS: usize = 16;
const MAX_GOLD_PER_ACTION: i32 = 100_000;
const MAX_PUPPET_DELAY_MS: i32 = rc_game::puppet::MAX_MIRROR_DELAY_MS as i32;
