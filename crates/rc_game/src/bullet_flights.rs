mod scan;
mod table;
mod tracker;

pub use scan::{SightedBullet, bullets_of_player, partner_effect_id, style_is_set};
pub use table::BulletFlightTable;
pub use tracker::BulletTracker;

pub const BULLET_SCAN_INTERVAL_MS: u64 = 100;
pub const BULLET_FLIGHT_STALE_MS: u64 = 600;
pub const BULLET_LOOK_REPEAT_EVERY_MESSAGES: u32 = 10;
pub const MAX_SIGHTED_BULLETS: usize = 4;
pub const MAX_PARTNER_BULLET_FLIGHTS: usize = 4;
pub const MAX_EXTRAPOLATION_MS: u64 = 250;
pub const BULLET_STATE_RUNNING: i32 = 0;
