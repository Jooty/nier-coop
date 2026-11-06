mod stub;

pub use rc_launcher_core::{
    ConnectionReport, LOBBY_MODE_CONTINUE, LOBBY_MODE_NEW_GAME, LauncherCore, LauncherEvent,
    LauncherNet, LinkStatus, LobbyReady, MAX_LOBBY_SLOT, NetError, NetState, NetStatus, PathKind,
    PlayerSettings, RttStats, SessionOptions,
};
pub use stub::StubNet;

pub const JOIN_PRECHECK_SECONDS: u32 = 3;
pub const DEFAULT_PLAYER_NAME: &str = rc_launcher_core::DEFAULT_PLAYER_NAME;
