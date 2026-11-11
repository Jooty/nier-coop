//! Host enemies' bullets: sent by the host, drawn on the guest as effects or through the game's
//! sprite batch (visual only).

mod send;
mod show;
mod sprite_draw;
mod tick;

pub use sprite_draw::install;
pub use tick::tick;

const ENEMY_BULLET_SUMMARY_INTERVAL_MS: u64 = 5000;
