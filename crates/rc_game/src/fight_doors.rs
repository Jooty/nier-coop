mod fight_door_table;
mod gate;
mod held_locks;

pub use fight_door_table::FIGHT_DOORS;
pub use gate::{is_fight_door, judge_fight_door_press};
pub use held_locks::HeldLocks;

pub const FIGHT_DOOR_PARTNER_RADIUS_UNITS: f32 = 4.0;
