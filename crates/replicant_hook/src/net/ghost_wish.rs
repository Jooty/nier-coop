use std::time::Duration;

use crate::net::NetState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GhostWish {
    Wanted,
    TurnedOff,
    NotConnected,
    PeerSilent,
    OwnPhaseUnknown,
    DifferentPhase,
}

impl GhostWish {
    pub fn why_not(self) -> Option<&'static str> {
        match self {
            Self::Wanted => None,
            Self::TurnedOff => Some("the ghost checkbox is off"),
            Self::NotConnected => Some("no other player is connected"),
            Self::PeerSilent => Some("the other player stopped sending"),
            Self::OwnPhaseUnknown => Some("your own phase is not readable (loading?)"),
            Self::DifferentPhase => Some("the other player is in another phase"),
        }
    }
}

pub fn decide_ghost(
    enabled: bool,
    link: &NetState,
    peer_age: Option<Duration>,
    peer_timeout: Duration,
    local_phase: Option<u16>,
    peer_phase: u16,
) -> GhostWish {
    if !enabled {
        return GhostWish::TurnedOff;
    }
    let Some(age) = peer_age.filter(|_| *link == NetState::Connected) else {
        return GhostWish::NotConnected;
    };
    if age > peer_timeout {
        return GhostWish::PeerSilent;
    }
    match local_phase {
        None => GhostWish::OwnPhaseUnknown,
        Some(phase) if phase != peer_phase => GhostWish::DifferentPhase,
        Some(_) => GhostWish::Wanted,
    }
}
