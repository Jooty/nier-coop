mod door_rules;
mod door_table;
mod flip_watch;
mod indoor_arrival;
mod own_door;

pub use door_rules::{
    DoorMoveFacts, door_by_id, door_entered_by, door_steps_for_a_move, door_with_start_point,
};
pub use door_table::{SIDE_VIEW_DOORS, SideViewDoor};
pub use flip_watch::SideViewWatch;
pub use indoor_arrival::{IndoorArrivalReading, IndoorArrivalWatch};
pub use own_door::OwnSideViewDoor;

pub const INDOOR_ARRIVAL_SETTLE_MS: u64 = 1_000;
