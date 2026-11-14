use core::f32::consts::{PI, TAU};

use crate::autopilot::{AUTOPILOT_CIRCLE_PERIOD_MS, AUTOPILOT_STICK_LENGTH, Autopilot};

/// The controller's speed and world facing, as the stick update writes them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StickFeed {
    pub speed: f32,
    pub world_facing: f32,
}

/// What the local hero's controller gets this tick; `None` while the player is not free.
pub fn autopilot_feed(setting: Autopilot, local_free: bool, now_ms: u64) -> Option<StickFeed> {
    if setting != Autopilot::Circle || !local_free {
        return None;
    }
    let turn = (now_ms % AUTOPILOT_CIRCLE_PERIOD_MS) as f32 / AUTOPILOT_CIRCLE_PERIOD_MS as f32;
    Some(StickFeed {
        speed: AUTOPILOT_STICK_LENGTH,
        world_facing: turn * TAU - PI,
    })
}
