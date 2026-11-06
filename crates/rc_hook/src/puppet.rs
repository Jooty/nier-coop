mod actor;
mod delay_setting;
mod error;
mod game_calls;
mod ghost;
mod mirror;
mod registry;

pub use delay_setting::{mirror_delay_ms, set_mirror_delay_ms};
pub use ghost::{ghost_enabled, ghost_status, set_ghost_enabled, tick_ghost};
pub use registry::{PuppetInfo, despawn, snapshot, spawn_mirror, tick};

use std::time::Duration;

const MAX_PUPPETS: usize = 8;
const GHOST_DELAY_MS: u64 = 100;
const GHOST_PEER_TIMEOUT: Duration = Duration::from_secs(3);
const GHOST_RESPAWN_COOLDOWN: Duration = Duration::from_secs(1);
