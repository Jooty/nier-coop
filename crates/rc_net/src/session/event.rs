use rc_proto::{Message, PlayerState, RejectReason};

use crate::transport::PeerId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeaveReason {
    Quit,
    TimedOut,
    ProtocolViolation,
    TransportClosed,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SessionEvent {
    PeerJoined {
        peer: PeerId,
        name: String,
        slot: u8,
    },
    PeerLeft {
        peer: PeerId,
        reason: LeaveReason,
    },
    PeerState {
        peer: PeerId,
        state: PlayerState,
    },
    PeerRelayed {
        peer: PeerId,
        message: Message,
    },
    Rejected {
        reason: RejectReason,
    },
    PeerRefused {
        peer: PeerId,
        reason: RejectReason,
    },
}
