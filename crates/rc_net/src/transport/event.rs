use crate::transport::PeerId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    UnreliableLatest,
    ReliableOrdered,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisconnectReason {
    Closed,
    TimedOut,
    Failed,
}

impl DisconnectReason {
    pub fn label(self) -> &'static str {
        match self {
            DisconnectReason::Closed => "closed by the other end",
            DisconnectReason::TimedOut => "timed out",
            DisconnectReason::Failed => "failed with a transport error",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransportEvent {
    PeerConnected(PeerId),
    PeerDisconnected(PeerId, DisconnectReason),
    Message {
        from: PeerId,
        channel: Channel,
        payload: Vec<u8>,
    },
}
