mod stub;
mod types;

pub use stub::StubNet;
pub use types::{ConnectionReport, LauncherNet, NetError, NetState, NetStatus, PathKind, RttStats};

pub const DEFAULT_TEST_SECONDS: u32 = 5;
