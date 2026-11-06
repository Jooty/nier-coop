use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::net::local_snapshot::LocalSnapshot;
use rc_proto::{PlayerState, RejectReason};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum NetState {
    #[default]
    WaitingForLauncher,
    LauncherLinked,
    Connected,
    Rejected(RejectReason),
    Failed(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemotePlayer {
    pub name: String,
    pub slot: u8,
    pub state: PlayerState,
    pub received_at: Instant,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct NetShared {
    pub state: NetState,
    pub link_port: Option<u16>,
    pub peer_name: Option<String>,
    pub rtt: Option<Duration>,
    pub direct_path: Option<bool>,
    pub remote: Option<RemotePlayer>,
    pub local: Option<LocalSnapshot>,
}

pub type SharedNet = Arc<Mutex<NetShared>>;

impl NetState {
    pub fn label(&self) -> String {
        match self {
            Self::WaitingForLauncher => "waiting for the launcher".to_owned(),
            Self::LauncherLinked => "launcher linked, no other player".to_owned(),
            Self::Connected => "connected".to_owned(),
            Self::Rejected(reason) => format!("rejected ({reason:?})"),
            Self::Failed(message) => format!("failed ({message})"),
        }
    }
}

impl RemotePlayer {
    pub fn age(&self) -> Duration {
        self.received_at.elapsed()
    }
}
