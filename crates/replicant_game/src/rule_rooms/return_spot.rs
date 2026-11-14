use crate::rule_rooms::RuleRoom;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReturnSpot {
    pub room: u8,
    pub position: [f32; 3],
    pub heading_degrees: f32,
    pub camera_status_kind: i32,
    pub camera_status_seconds: f64,
}

pub static RETURN_SPOTS: [ReturnSpot; 8] = [
    ReturnSpot {
        room: 1,
        position: [20.0, 12.0, 74.0],
        heading_degrees: 90.0,
        camera_status_kind: 10,
        camera_status_seconds: 3.0,
    },
    ReturnSpot {
        room: 2,
        position: [-23.0, 12.0, 74.0],
        heading_degrees: -90.0,
        camera_status_kind: 0,
        camera_status_seconds: 0.0,
    },
    ReturnSpot {
        room: 3,
        position: [-55.0, 8.0, 42.13],
        heading_degrees: 90.0,
        camera_status_kind: 0,
        camera_status_seconds: 0.0,
    },
    ReturnSpot {
        room: 4,
        position: [22.0, 8.0, 42.0],
        heading_degrees: 90.0,
        camera_status_kind: 0,
        camera_status_seconds: 0.0,
    },
    ReturnSpot {
        room: 5,
        position: [20.0, 4.0, 6.0],
        heading_degrees: 90.0,
        camera_status_kind: 0,
        camera_status_seconds: 0.0,
    },
    ReturnSpot {
        room: 6,
        position: [-22.0, 4.0, 6.0],
        heading_degrees: -90.0,
        camera_status_kind: 0,
        camera_status_seconds: 0.0,
    },
    ReturnSpot {
        room: 7,
        position: [-23.85, 0.0, -30.0],
        heading_degrees: 90.0,
        camera_status_kind: 14,
        camera_status_seconds: 0.0,
    },
    ReturnSpot {
        room: 8,
        position: [19.15, -0.05, -22.0],
        heading_degrees: 90.0,
        camera_status_kind: 0,
        camera_status_seconds: 0.0,
    },
];

pub fn return_spot_of(room: &RuleRoom) -> Option<&'static ReturnSpot> {
    RETURN_SPOTS.iter().find(|spot| spot.room == room.room)
}
