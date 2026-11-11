mod door_rules;
mod door_table;
mod flip_watch;
mod indoor_arrival;
mod own_door;
mod view_off;

pub use door_rules::{
    DoorMoveAction, DoorMoveFacts, door_action_for_a_move, door_by_id, door_entered_by,
    door_with_start_point,
};
pub use door_table::{SIDE_VIEW_DOORS, SideViewDoor};
pub use flip_watch::SideViewWatch;
pub use indoor_arrival::{ArrivalViewFix, IndoorArrivalReading, IndoorArrivalWatch};
pub use own_door::OwnSideViewDoor;
pub use view_off::{ViewOffArg, ViewOffCall, view_off_calls};

pub const INDOOR_ARRIVAL_SETTLE_MS: u64 = 1_000;
pub const SIDE_VIEW_MODE_OFF_NATIVE_RVA: u32 = 0x70_6ED0;
pub const SET_ACTION_MODE_NATIVE_RVA: u32 = 0x6F_40B0;
pub const SET_CAMERA_LEVER_MODE_NATIVE_RVA: u32 = 0x6F_74A0;
pub const SET_CAMERA_COLLISION_NATIVE_RVA: u32 = 0x6F_7440;
pub const CHANGE_CAMERA_STATUS_NATIVE_RVA: u32 = 0x6E_4E70;
pub const CHANGE_CAMERA_NATIVE_RVA: u32 = 0x6E_4B50;
pub const MINI_MAP_ACTIVE_NATIVE_RVA: u32 = 0x6E_E130;
pub const TERRAIN_PARTS_VISIBLE_NATIVE_RVA: u32 = 0x70_BB00;
pub const OPEN_PLAYER_GAUGES_SCRIPT_FUNCTION: &str = "LIB_SET_OPEN_2D_GAUGE_FAST";
