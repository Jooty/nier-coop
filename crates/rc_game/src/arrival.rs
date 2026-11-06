mod fall;
mod follow_arrival;

pub use fall::{FallGuard, FallInputs};
pub use follow_arrival::{ArrivalGuard, ArrivalInputs, ArrivalStep, PlaceReason};

pub const ARRIVAL_SETTLE_MS: u64 = 1000;
pub const ARRIVAL_RETRY_MS: u64 = 750;
pub const ARRIVAL_GIVE_UP_MS: u64 = 30_000;
pub const ARRIVAL_ACCEPTABLE_DISTANCE: f32 = 6.0;
pub const FALL_NO_GROUND_MS: u64 = 1500;
pub const FALL_DEPTH_BELOW_REFERENCE: f32 = 15.0;
pub const FALL_RECOVERY_COOLDOWN_MS: u64 = 4000;
