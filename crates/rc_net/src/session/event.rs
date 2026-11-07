use rc_proto::{Message, PlayerState, RejectReason};

use crate::transport::PeerId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeaveReason {
    Quit,
    TimedOut,
    ProtocolViolation,
    TransportClosed,
}

impl LeaveReason {
    pub fn label(self) -> &'static str {
        match self {
            LeaveReason::Quit => "the other player closed the link",
            LeaveReason::TimedOut => "no traffic from the other player (timed out)",
            LeaveReason::ProtocolViolation => "the other player sent something invalid",
            LeaveReason::TransportClosed => "the network link closed or failed",
        }
    }
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
