//! Host enemies' bullets: sent by the host, drawn on the guest as effects or through the game's
//! sprite batch (visual only).

mod script_effects;
mod send;
mod show;
mod sprite_draw;
mod tick;

pub use script_effects::install as install_script_effects;
pub(crate) use script_effects::{admit_bullet_script_effect, in_enemy_bullet_effect_command};
pub use sprite_draw::install;
pub use tick::tick;

const ENEMY_BULLET_SUMMARY_INTERVAL_MS: u64 = 5000;
