use std::time::Duration;

use rc_proto::{GameReadiness, RejectReason};

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum ConnectionState {
    #[default]
    Idle,
    Hosting,
    Joining,
    Connected,
    Rejected(RejectReason),
    Failed(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LauncherEvent {
    Hosting { ticket: String },
    Joining,
    PeerJoined { name: String },
    PeerLeft,
    PeerRefused { reason: RejectReason },
    Rejected { reason: RejectReason },
    GameLinked,
    GameUnlinked,
    OwnGameReadiness { readiness: GameReadiness },
    PartnerGameReadiness { readiness: GameReadiness },
    Failed { message: String },
    Stopped,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct LinkStatus {
    pub state: ConnectionState,
    pub ticket: Option<String>,
    pub peer_name: Option<String>,
    pub rtt: Option<Duration>,
    pub direct_path: Option<bool>,
    pub game_linked: bool,
    pub peer_readiness: GameReadiness,
    pub link_port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum NetState {
    #[default]
    Idle,
    Hosting {
        ticket: String,
    },
    Joining,
    Connected {
        peer_name: String,
    },
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NetStatus {
    pub state: NetState,
}
