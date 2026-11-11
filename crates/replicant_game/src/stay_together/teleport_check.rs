use crate::stay_together::distance_between;

pub const TELEPORT_ARRIVED_UNITS: f32 = 1.0;
pub const TELEPORT_GIVE_UP_MS: u64 = 1500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeleportCheck {
    Arrived,
    Waiting,
    Missed,
}

pub fn check_teleport(target: [f32; 3], now: [f32; 3], elapsed_ms: u64) -> TeleportCheck {
    let near = distance_between(target, now).is_some_and(|gap| gap <= TELEPORT_ARRIVED_UNITS);
    if near {
        TeleportCheck::Arrived
    } else if elapsed_ms >= TELEPORT_GIVE_UP_MS {
        TeleportCheck::Missed
    } else {
        TeleportCheck::Waiting
    }
}
