mod matching;
mod mirror;
mod roster;
mod scan;
mod shadow_matrix;
mod sighting;
mod snapshot;

pub use matching::{EnemyMatch, match_enemies};
pub use mirror::{EnemyMirror, MirroredEnemy};
pub use roster::{EnemyRoster, RosterUpdate};
pub use scan::scan_enemies;
pub use shadow_matrix::shadow_matrix;
pub use sighting::EnemySighting;
pub use snapshot::{enemy_state, split_snapshot};

pub const ENEMY_MIRROR_STALE_MS: u64 = 1000;
pub const ENEMY_MIRROR_DELAY_MS: u64 = 100;
