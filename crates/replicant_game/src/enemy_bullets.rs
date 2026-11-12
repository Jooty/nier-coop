//! Bullets the host's enemies fire, scanned on the host and drawn on the guest as effects or
//! through the game's sprite batch.

mod board;
mod kinds;
mod laser_beam;
mod refusals;
mod scan;
mod script_bullet;
mod sender;
mod shown_effects;
mod sprite_frames;
mod uncopied;

pub use board::{EnemyBulletBoard, EnemyBulletStep};
pub use kinds::{EnemyBulletKinds, SpriteBulletKinds};
pub use laser_beam::{
    BeamEnd, LaserBeam, LaserBeamTally, beam_end_text, longest_axis, read_laser_beam,
};
pub use refusals::FlightRefusals;
pub use scan::{
    EnemyBullet, EnemyBulletCensus, EnemyBulletScan, UncopiedScriptBullet, bullets_of_enemies,
    scan_enemy_bullets,
};
pub use script_bullet::{
    NotCopyable, UncopiedWhy, read_script_bullet, script_bullet_is_laser, why_not_copied,
};
pub use sender::EnemyBulletSender;
pub use shown_effects::ShownEffectTally;
pub use sprite_frames::{first_sprite_frame, next_sprite_frame};
pub use uncopied::{UncopiedTally, bullet_script_effects_line, uncopied_why_text};

pub const ENEMY_BULLET_SCAN_INTERVAL_MS: u64 = 50;
pub const ENEMY_BULLET_STALE_MS: u64 = 600;
pub const MAX_SIGHTED_ENEMY_BULLETS: usize = 48;
pub const MAX_SIGHTED_ENEMY_SPRITE_BULLETS: usize = 80;
pub const MAX_SHOWN_ENEMY_BULLETS: usize = 128;
pub const MAX_ENEMY_BULLET_MOVES_SENT_PER_SCAN: usize = 24;
pub const MAX_ENEMY_BULLET_KINDS_LOGGED: usize = 64;
pub const MAX_SPRITE_BULLET_KINDS_LOGGED: usize = 16;
pub const BULLET_SPRITE_FRAME_COUNT: u16 = 64;
pub const MAX_UNCOPIED_SCRIPT_BULLETS_READ: usize = 32;
pub const SCRIPT_BULLET_LASER_FLAG_OFFSET: u64 = 0x1698;
pub const SCRIPT_BULLET_EXECUTE_COMMAND_RVA: u32 = 0x33_0FA0;
pub const SCRIPT_BULLET_COMMAND_RECEIVER_OFFSET: u64 = 0x1900;
pub const SCRIPT_BULLET_COMMANDS_THAT_MAKE_ONE_SHOT_EFFECTS: [i32; 2] = [0x36, 0x37];
pub const MAX_BULLET_SCRIPT_EFFECTS_SENT_PER_SECOND: u32 = 20;
pub const MAX_BULLET_SCRIPT_EFFECT_NAMES_LOGGED: usize = 8;
pub const MAX_LASER_BEAM_KINDS_NAMED: usize = 16;
pub const MAX_SHOWN_BULLET_EFFECTS_LOGGED: usize = 8;
