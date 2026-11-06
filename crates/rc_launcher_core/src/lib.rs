mod bridge;
mod config;
mod core;
mod error;
mod event;
mod launcher_net;
mod probe;
mod report;

use std::time::Duration;

pub use config::{LauncherConfig, TransportFactory, iroh_factory};
pub use core::LauncherCore;
pub use error::NetError;
pub use event::{ConnectionState, LauncherEvent, LinkStatus, NetState, NetStatus};
pub use launcher_net::LauncherNet;
pub use rc_proto::SessionOptions;
pub use report::{ConnectionReport, PathKind, Quality, RttStats, Verdict, judge};

pub const WORKER_WAKE_INTERVAL: Duration = Duration::from_millis(4);
pub const PATH_INFO_INTERVAL: Duration = Duration::from_millis(500);
pub const PROBE_SAMPLE_INTERVAL: Duration = Duration::from_millis(200);
pub const PROBE_MIN_SECONDS: u32 = 2;
pub const PROBE_MAX_SECONDS: u32 = 120;
pub const COMMAND_REPLY_TIMEOUT: Duration = Duration::from_secs(40);
pub const DEFAULT_PLAYER_NAME: &str = "Nier";

pub const INVALID_TICKET_TEXT: &str = "that ticket is not valid";
