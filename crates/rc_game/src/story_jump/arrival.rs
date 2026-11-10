use rc_proto::StoryJumpPoint;

use crate::story_jump::LEVEL_CHECK_SETTLE_MS;

const ARENA_HOST_POSITION: [f32; 3] = [-2.2, 146.8, -8.6];
const ARENA_GUEST_SIDE_OFFSET_UNITS: f32 = 2.0;
const OUTSIDE_THE_HOUSE_HOST_POSITION: [f32; 3] = [30.8, 14.0, 7.1];
const OUTSIDE_THE_HOUSE_GUEST_POSITION: [f32; 3] = [30.9, 14.0, 8.5];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArrivalSpot {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub fn arrival_spot(point: StoryJumpPoint, host: bool) -> Option<ArrivalSpot> {
    match point {
        StoryJumpPoint::AtShrineBoss => {
            let [x, y, z] = ARENA_HOST_POSITION;
            let side = if host {
                0.0
            } else {
                ARENA_GUEST_SIDE_OFFSET_UNITS
            };
            Some(ArrivalSpot { x: x + side, y, z })
        }
        StoryJumpPoint::HomeAfterShrineBoss => {
            let [x, y, z] = if host {
                OUTSIDE_THE_HOUSE_HOST_POSITION
            } else {
                OUTSIDE_THE_HOUSE_GUEST_POSITION
            };
            Some(ArrivalSpot { x, y, z })
        }
        _ => None,
    }
}

pub fn arrival_is_due(
    phase_at_apply: &str,
    phase_now: &str,
    field_was_left: bool,
    field_ready: bool,
    ms_since_phase_change: u64,
) -> bool {
    let phase_changed = phase_at_apply != phase_now || field_was_left;
    phase_changed && field_ready && ms_since_phase_change >= LEVEL_CHECK_SETTLE_MS
}
