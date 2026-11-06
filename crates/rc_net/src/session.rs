mod auth;
mod config;
mod driver;
mod event;
mod peer;

use std::time::Duration;

pub use auth::{fresh_nonce, password_proof, proofs_match};
pub use config::{Role, SessionConfig};
pub use driver::{PingCounts, Session};
pub use event::{LeaveReason, SessionEvent};

pub const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(1);
pub const PEER_TIMEOUT: Duration = Duration::from_secs(5);
pub const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
pub const REFUSAL_LINGER: Duration = Duration::from_millis(500);
pub const ATTEMPT_WINDOW: Duration = Duration::from_secs(60);
pub const PEER_MAX_FAILURES: usize = 3;
pub const GLOBAL_MAX_FAILURES: usize = 10;
