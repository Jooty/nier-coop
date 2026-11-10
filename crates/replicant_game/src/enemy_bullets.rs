//! Bullets the host's enemies fire, scanned on the host and drawn as effects on the guest.

mod board;
mod kinds;
mod scan;
mod sender;

pub use board::{EnemyBulletBoard, EnemyBulletStep};
pub use kinds::EnemyBulletKinds;
pub use scan::{EnemyBullet, bullets_of_enemies};
pub use sender::EnemyBulletSender;

pub const ENEMY_BULLET_SCAN_INTERVAL_MS: u64 = 50;
pub const ENEMY_BULLET_STALE_MS: u64 = 600;
pub const MAX_SIGHTED_ENEMY_BULLETS: usize = 16;
pub const MAX_SHOWN_ENEMY_BULLETS: usize = 16;
pub const MAX_ENEMY_BULLET_KINDS_LOGGED: usize = 64;
