mod action;
mod queue;
mod tick_hook;

pub use action::Action;
pub use queue::enqueue;
pub use tick_hook::install;

const MAX_QUEUED_ACTIONS: usize = 16;
