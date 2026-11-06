mod stub;

pub use rc_launcher_core::{
    ConnectionReport, LauncherCore, LauncherEvent, LauncherNet, LinkStatus, NetError, NetState,
    NetStatus, PathKind, RttStats,
};
pub use stub::StubNet;

pub const DEFAULT_TEST_SECONDS: u32 = 5;
pub const DEFAULT_PLAYER_NAME: &str = rc_launcher_core::DEFAULT_PLAYER_NAME;
