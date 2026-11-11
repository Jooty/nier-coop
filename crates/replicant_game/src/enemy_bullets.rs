//! Bullets the host's enemies fire, scanned on the host and drawn as effects on the guest.

mod board;
mod kinds;
mod scan;
mod script_bullet;
mod sender;

pub use board::{EnemyBulletBoard, EnemyBulletStep};
pub use kinds::EnemyBulletKinds;
pub use scan::{
    EnemyBullet, EnemyBulletCensus, EnemyBulletScan, bullets_of_enemies, scan_enemy_bullets,
};
pub use script_bullet::{
    MAX_SPRITE_BULLET_KINDS_LOGGED, NotCopyable, SPRITE_EFFECT_OBJECT_BYTES_LOGGED,
    SpriteBulletKinds, SpriteBulletSample, read_script_bullet, read_sprite_bullet_hit_id,
    read_sprite_bullet_sample,
};
pub use sender::EnemyBulletSender;

pub const ENEMY_BULLET_SCAN_INTERVAL_MS: u64 = 50;
pub const ENEMY_BULLET_STALE_MS: u64 = 600;
pub const MAX_SIGHTED_ENEMY_BULLETS: usize = 48;
pub const MAX_SHOWN_ENEMY_BULLETS: usize = 48;
pub const MAX_ENEMY_BULLET_MOVES_SENT_PER_SCAN: usize = 16;
pub const MAX_ENEMY_BULLET_KINDS_LOGGED: usize = 64;
