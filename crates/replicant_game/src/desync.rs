mod reporter;
mod roster;

pub use reporter::DesyncReporter;
pub use roster::{ids_hash, live_group_rosters};

pub const DESYNC_DIGEST_INTERVAL_MS: u64 = 5000;
pub const DESYNC_IN_STEP_INTERVAL_MS: u64 = 60_000;
