mod auto_config;
mod ghost_wish;
mod link_state;
mod local_snapshot;
mod net_thread;
mod replicant_net;
mod shared;

#[cfg(windows)]
mod game_side;
#[cfg(windows)]
mod global;

pub use auto_config::{
    AUTO_CONFIG_FILE_NAME, AutoConfig, AutoConfigError, BLOCK_SAVES_ENV, BOOT_SLOT_ENV,
    DEFAULT_LOG_FILE_NAME, LINK_ENV, LOG_ENV, NAME_ENV, NEW_GAME_NAME_ENV, NEW_GAME_SLOT_ENV,
    resolve_log_path,
};
pub use ghost_wish::{GhostWish, decide_ghost};
pub use link_state::ReplicantLinkState;
pub use local_snapshot::LocalSnapshot;
pub use net_thread::NetThread;
pub use nt_hook::net::{
    MAX_RELAYED_INBOX, NetState, SEND_INTERVAL, WORKER_WAKE_INTERVAL, net_clock_ms,
};
pub use replicant_net::ReplicantNet;
pub use shared::{NetShared, NetSharedExt, RemotePlayer, SharedNet};

#[cfg(windows)]
pub use game_side::{publish_local_snapshot, quit_when_asked, report_readiness, start_auto};
#[cfg(windows)]
pub use global::{handle, start_link};
