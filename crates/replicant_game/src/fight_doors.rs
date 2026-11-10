mod fight_door_table;
mod fight_lock;
mod held_locks;

pub use fight_door_table::FIGHT_DOORS;
pub use fight_lock::{
    FightLockVerdict, HoldReason, LockOrigin, fight_door_in_zone, is_fight_door, judge_door_lock,
};
pub use held_locks::HeldLocks;

pub const INJECTED_RUN_LOCK_WINDOW_MS: u64 = 5000;
