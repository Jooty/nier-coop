mod actor;
mod delay_setting;
mod error;
mod game_calls;
mod mirror;
mod registry;

pub use delay_setting::{mirror_delay_ms, set_mirror_delay_ms};
pub use registry::{PuppetInfo, despawn, snapshot, spawn_mirror, tick};

const MAX_PUPPETS: usize = 8;
