use serde::{Deserialize, Serialize};

use crate::travel::{JumpRole, TravelPoint};

/// What one player's game asks of the other's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeleportRequest {
    ComeToSender,
    FollowJump { point: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncomingTeleport {
    GoToPartner,
    FollowJump(TravelPoint),
    IgnoredJumpOfANonHost,
    IgnoredUnknownPoint(u8),
}

impl TeleportRequest {
    pub fn follow(point: TravelPoint) -> Self {
        Self::FollowJump {
            point: point.index(),
        }
    }

    pub fn is_valid(self) -> bool {
        match self {
            Self::ComeToSender => true,
            Self::FollowJump { point } => TravelPoint::from_index(u32::from(point)).is_some(),
        }
    }
}

impl IncomingTeleport {
    pub fn ignored_line(self) -> Option<String> {
        match self {
            Self::GoToPartner | Self::FollowJump(_) => None,
            Self::IgnoredJumpOfANonHost => Some(
                "travel: ignored a jump message, only a guest follows and only the host sends one"
                    .to_owned(),
            ),
            Self::IgnoredUnknownPoint(point) => Some(format!(
                "travel: ignored a jump message for the unknown point number {point}"
            )),
        }
    }
}

/// `local_role` is this game's role, so a jump is followed only by a guest.
pub fn handle_teleport_request(local_role: JumpRole, request: TeleportRequest) -> IncomingTeleport {
    match request {
        TeleportRequest::ComeToSender => IncomingTeleport::GoToPartner,
        TeleportRequest::FollowJump { point } => {
            match (local_role, TravelPoint::from_index(u32::from(point))) {
                (_, None) => IncomingTeleport::IgnoredUnknownPoint(point),
                (JumpRole::Guest, Some(point)) => IncomingTeleport::FollowJump(point),
                (JumpRole::Host | JumpRole::Solo, Some(_)) => {
                    IncomingTeleport::IgnoredJumpOfANonHost
                }
            }
        }
    }
}
