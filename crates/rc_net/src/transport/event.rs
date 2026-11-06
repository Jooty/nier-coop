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
