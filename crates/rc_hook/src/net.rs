mod auto_config;
mod ghost_wish;
mod local_snapshot;
mod net_thread;
mod shared;
mod worker;

#[cfg(windows)]
mod game_side;
#[cfg(windows)]
mod global;

use std::time::Duration;

pub use auto_config::{
    AUTO_CONFIG_FILE_NAME, AutoConfig, AutoConfigError, BLOCK_SAVES_ENV, BOOT_SLOT_ENV, LINK_ENV,
    NAME_ENV,
};
pub use ghost_wish::{GhostWish, decide_ghost};
pub use local_snapshot::LocalSnapshot;
pub use net_thread::NetThread;
pub use shared::{NetShared, NetState, RemotePlayer, SharedNet};

#[cfg(windows)]
pub use game_side::{publish_local_snapshot, start_auto};
#[cfg(windows)]
pub use global::{handle, start_link};

pub const SEND_INTERVAL: Duration = Duration::from_millis(33);
pub const WORKER_WAKE_INTERVAL: Duration = Duration::from_millis(4);
