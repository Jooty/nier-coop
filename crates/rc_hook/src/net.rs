mod auto_config;
mod ghost_wish;
mod local_snapshot;
mod net_thread;
mod readiness_log;
mod shared;
mod worker;

#[cfg(windows)]
mod game_side;
#[cfg(windows)]
mod global;

use std::time::Duration;

pub use auto_config::{
    AUTO_CONFIG_FILE_NAME, AutoConfig, AutoConfigError, BLOCK_SAVES_ENV, BOOT_SLOT_ENV,
    DEFAULT_LOG_FILE_NAME, LINK_ENV, LOG_ENV, NAME_ENV, NEW_GAME_NAME_ENV, NEW_GAME_SLOT_ENV,
    resolve_log_path,
};
pub use ghost_wish::{GhostWish, decide_ghost};
pub use local_snapshot::LocalSnapshot;
pub use net_thread::NetThread;
pub use shared::{NetShared, NetState, RemotePlayer, SharedNet, net_clock_ms};

#[cfg(windows)]
pub use game_side::{publish_local_snapshot, report_readiness, start_auto};
#[cfg(windows)]
pub use global::{handle, start_link};

pub const SEND_INTERVAL: Duration = Duration::from_millis(33);
pub const WORKER_WAKE_INTERVAL: Duration = Duration::from_millis(4);
pub const MAX_RELAYED_INBOX: usize = 512;
