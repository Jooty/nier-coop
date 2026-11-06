use std::time::Duration;

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum NetError {
    #[error("{0}")]
    Failed(String),
    #[error("networking is not available in this build")]
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathKind {
    Direct,
    Relay,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RttStats {
    pub min: Duration,
    pub avg: Duration,
    pub max: Duration,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConnectionReport {
    pub reachable: bool,
    pub path: PathKind,
    pub rtt: Option<RttStats>,
    pub loss_percent: f32,
    pub verdict: String,
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

pub trait LauncherNet: Send + Sync {
    fn host(&self, password: Option<&str>) -> Result<String, NetError>;
    fn join(&self, ticket: &str, password: Option<&str>) -> Result<(), NetError>;
    fn test_connection(
        &self,
        ticket: &str,
        password: Option<&str>,
        seconds: u32,
    ) -> Result<ConnectionReport, NetError>;
    fn leave(&self);
    fn status(&self) -> NetStatus;
}
