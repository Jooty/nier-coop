//! Bullets the host's enemies fire, scanned on the host and drawn on the guest as effects or
//! through the game's sprite batch.

mod board;
mod kinds;
mod scan;
mod script_bullet;
mod sender;
mod sprite_frames;

pub use board::{EnemyBulletBoard, EnemyBulletStep};
pub use kinds::{EnemyBulletKinds, SpriteBulletKinds};
pub use scan::{
    EnemyBullet, EnemyBulletCensus, EnemyBulletScan, bullets_of_enemies, scan_enemy_bullets,
};
pub use script_bullet::{NotCopyable, read_script_bullet};
pub use sender::EnemyBulletSender;
pub use sprite_frames::{first_sprite_frame, next_sprite_frame};

pub const ENEMY_BULLET_SCAN_INTERVAL_MS: u64 = 50;
pub const ENEMY_BULLET_STALE_MS: u64 = 600;
pub const MAX_SIGHTED_ENEMY_BULLETS: usize = 48;
pub const MAX_SIGHTED_ENEMY_SPRITE_BULLETS: usize = 80;
pub const MAX_SHOWN_ENEMY_BULLETS: usize = 128;
pub const MAX_ENEMY_BULLET_MOVES_SENT_PER_SCAN: usize = 24;
pub const MAX_ENEMY_BULLET_KINDS_LOGGED: usize = 64;
pub const MAX_SPRITE_BULLET_KINDS_LOGGED: usize = 16;
pub const BULLET_SPRITE_FRAME_COUNT: u16 = 64;
