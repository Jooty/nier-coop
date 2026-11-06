mod auto_config;
mod command;
mod local_snapshot;
mod net_thread;
mod shared;
mod worker;

#[cfg(windows)]
mod game_side;
#[cfg(windows)]
mod global;

use std::time::Duration;

pub use auto_config::{AUTO_CONFIG_FILE_NAME, AutoConfig, AutoConfigError, AutoMode};
pub use command::NetCommand;
pub use local_snapshot::LocalSnapshot;
pub use net_thread::NetThread;
pub use shared::{NetShared, NetState, RemotePlayer, SharedNet};

#[cfg(windows)]
pub use game_side::{publish_local_snapshot, start_auto};
#[cfg(windows)]
pub use global::{handle, set_player_name};

pub const DEFAULT_PLAYER_NAME: &str = "Nier";
pub const SEND_INTERVAL: Duration = Duration::from_millis(33);
pub const WORKER_WAKE_INTERVAL: Duration = Duration::from_millis(4);
pub const LINK_STATS_INTERVAL: Duration = Duration::from_millis(250);
