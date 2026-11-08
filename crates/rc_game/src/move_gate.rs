mod holds;
mod judge;
mod moving_events;

pub use holds::{HeldMove, MoveHolds, MoveRelease, MoveReleased, StartAnswer};
pub use judge::{MoveAsk, MoveVerdict, judge_move, moves_the_player};
pub use moving_events::MOVING_EVENT_BASES;

pub const MOVE_HOLD_GIVE_UP_MS: u64 = 120_000;
pub const MOVE_HOLD_REMINDER_MS: u64 = 5_000;
pub const MOVE_RELEASE_PASS_MS: u64 = 10_000;
