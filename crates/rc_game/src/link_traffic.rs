use std::time::Duration;

mod refusals;
mod tracker;

pub use refusals::RefusalLog;
pub use tracker::LinkTraffic;

pub const REFUSAL_LOG_INTERVAL: Duration = Duration::from_secs(60);
