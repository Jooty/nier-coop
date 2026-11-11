mod table;
mod tracker;
mod watched;

pub use table::WATCHED_CALLBACK_TABLE;
pub use tracker::{OneSidedTracker, Side, Verdict};
pub use watched::{is_watched_callback, one_sided_log_line};

pub const BOTH_RAN_WINDOW_MS: u64 = 10_000;
pub const MAX_PENDING_WATCHED_CALLBACKS: usize = 64;
