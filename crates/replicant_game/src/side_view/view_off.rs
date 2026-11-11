use crate::side_view::{
    CHANGE_CAMERA_NATIVE_RVA, CHANGE_CAMERA_STATUS_NATIVE_RVA, MINI_MAP_ACTIVE_NATIVE_RVA,
    SET_ACTION_MODE_NATIVE_RVA, SET_CAMERA_COLLISION_NATIVE_RVA, SET_CAMERA_LEVER_MODE_NATIVE_RVA,
    SIDE_VIEW_MODE_OFF_NATIVE_RVA, SideViewDoor, TERRAIN_PARTS_VISIBLE_NATIVE_RVA,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewOffArg {
    Int(i32),
    Real(f64),
    Text(&'static str),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ViewOffCall {
    pub native_rva: u32,
    pub args: Vec<ViewOffArg>,
}

/// The game's own natives that end a side view, in the order a door's exit calls them, with
/// none of the exit's story work. `left_door` adds what belongs to that place in the same area:
/// its outer shell made visible again and its outdoor camera status.
pub fn view_off_calls(left_door: Option<&SideViewDoor>) -> Vec<ViewOffCall> {
    let call = |native_rva, args: &[ViewOffArg]| ViewOffCall {
        native_rva,
        args: args.to_vec(),
    };
    let mut calls = vec![
        call(
            SET_CAMERA_LEVER_MODE_NATIVE_RVA,
            &[ViewOffArg::Int(1), ViewOffArg::Int(1)],
        ),
        call(SET_CAMERA_COLLISION_NATIVE_RVA, &[ViewOffArg::Int(1)]),
    ];
    if let Some(door) = left_door {
        for part in door.shell_terrain_parts {
            calls.push(call(
                TERRAIN_PARTS_VISIBLE_NATIVE_RVA,
                &[ViewOffArg::Text(part)],
            ));
        }
        calls.push(call(
            CHANGE_CAMERA_STATUS_NATIVE_RVA,
            &[
                ViewOffArg::Int(door.outdoor_camera_status),
                ViewOffArg::Real(0.0),
                ViewOffArg::Int(1),
            ],
        ));
    }
    calls.extend([
        call(
            CHANGE_CAMERA_NATIVE_RVA,
            &[
                ViewOffArg::Int(0),
                ViewOffArg::Real(1.0),
                ViewOffArg::Int(1),
            ],
        ),
        call(SET_ACTION_MODE_NATIVE_RVA, &[ViewOffArg::Int(0)]),
        call(SIDE_VIEW_MODE_OFF_NATIVE_RVA, &[]),
        call(MINI_MAP_ACTIVE_NATIVE_RVA, &[ViewOffArg::Int(0)]),
    ]);
    calls
}
