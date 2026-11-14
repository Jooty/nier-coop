use crate::rule_rooms::{GUEST_ALREADY_BESIDE_THE_HOST_UNITS, RuleRoom, room_name};
use crate::stay_together::distance_between;

/// What the host's start of a rule room does with the guest: a guest away from the host is moved
/// beside the host, inside the room, before the room's first camera and fade are judged here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TakeIn {
    AlreadyBeside {
        units: f32,
    },
    MoveBeside {
        host_at: [f32; 3],
        units: Option<f32>,
    },
    HostNotKnown,
}

impl TakeIn {
    pub fn judge(guest_at: Option<[f32; 3]>, host_at: Option<[f32; 3]>) -> Self {
        let Some(host_at) = host_at else {
            return Self::HostNotKnown;
        };
        let units = guest_at.and_then(|guest_at| distance_between(guest_at, host_at));
        match units {
            Some(units) if units <= GUEST_ALREADY_BESIDE_THE_HOST_UNITS => {
                Self::AlreadyBeside { units }
            }
            _ => Self::MoveBeside { host_at, units },
        }
    }

    pub fn line(self, room: &RuleRoom) -> String {
        let room = room_name(room);
        match self {
            Self::AlreadyBeside { units } => format!(
                "rule room: {room} starts from the host's entry, you are already beside the host ({units:.1} units), not moved"
            ),
            Self::MoveBeside { host_at, units } => format!(
                "rule room: {room} starts from the host's entry, you are taken in beside the host at ({:.1}, {:.1}, {:.1}), {} away",
                host_at[0],
                host_at[1],
                host_at[2],
                units.map_or_else(
                    || "an unknown distance".to_owned(),
                    |units| format!("{units:.1} units")
                )
            ),
            Self::HostNotKnown => format!(
                "rule room: {room} starts from the host's entry, but the host's place is not known here, you are not moved"
            ),
        }
    }
}

pub fn take_in_failed_line(room: &RuleRoom, reason: &str) -> String {
    format!(
        "rule room: {} starts from the host's entry, you could not be taken in ({reason})",
        room_name(room)
    )
}
