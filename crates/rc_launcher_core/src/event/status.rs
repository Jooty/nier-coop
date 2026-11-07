use std::time::Duration;

use rc_proto::{GameReadiness, LobbyReady, PlayerSettings, RejectReason, SessionOptions};

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
    PeerLeft { detail: String },
    PeerRefused { reason: RejectReason },
    Rejected { reason: RejectReason },
    GameLinked,
    GameUnlinked { detail: String },
    OwnGameReadiness { readiness: GameReadiness },
    PartnerGameReadiness { readiness: GameReadiness },
    GameChangedSettings { settings: PlayerSettings },
    SessionOptionsChanged { options: SessionOptions },
    LobbyPartnerReady { ready: LobbyReady },
    LobbyStartReceived { mode: u8 },
    MessageRefused { kind: String, reason: String },
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
    pub guest: bool,
    pub peer_readiness: GameReadiness,
    pub peer_lobby: Option<LobbyReady>,
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
