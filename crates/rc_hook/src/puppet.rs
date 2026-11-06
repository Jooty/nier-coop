mod actor;
mod error;
mod game_calls;
mod ghost;
mod registry;
mod remote_driver;

pub use ghost::{ghost_enabled, ghost_status, set_ghost_enabled, tick_ghost};
pub use registry::{despawn, tick};

use std::time::Duration;

const MAX_PUPPETS: usize = 8;
const GHOST_DELAY_MS: u64 = 100;
const GHOST_PEER_TIMEOUT: Duration = Duration::from_secs(3);
const GHOST_RESPAWN_COOLDOWN: Duration = Duration::from_secs(1);
