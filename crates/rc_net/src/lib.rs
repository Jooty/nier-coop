#[cfg(feature = "iroh")]
mod iroh_adapter;
mod local_link;
mod loopback;
mod session;
mod transport;

#[cfg(feature = "iroh")]
pub use iroh_adapter::{IrohConfig, IrohTransport};
pub use local_link::{
    LINK_ADDRESS, LINK_GAME_PEER, LINK_INVITE_PREFIX, LINK_LAUNCHER_PEER, LINK_VERSION,
    LocalLinkTransport,
};
pub use loopback::{LoopbackNetwork, LoopbackTransport};
pub use session::{
    ATTEMPT_WINDOW, GLOBAL_MAX_FAILURES, HANDSHAKE_TIMEOUT, KEEPALIVE_INTERVAL, LeaveReason,
    PEER_MAX_FAILURES, PEER_TIMEOUT, PingCounts, REFUSAL_LINGER, Role, Session, SessionConfig,
    SessionEvent, fresh_nonce, password_proof, proofs_match,
};
pub use transport::{
    Channel, DisconnectReason, Invite, LinkStats, MAX_INVITE_BYTES, MAX_PAYLOAD_BYTES, NetError,
    PeerId, Transport, TransportEvent,
};
