use rc_proto::StoryJumpPoint;

const ARENA_HOST_POSITION: [f32; 3] = [-2.2, 146.8, -8.6];
const ARENA_GUEST_SIDE_OFFSET_UNITS: f32 = 2.0;

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
        _ => None,
    }
}
