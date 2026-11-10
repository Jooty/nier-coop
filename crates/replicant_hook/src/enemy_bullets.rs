//! Host enemies' bullets: sent by the host, drawn as effects on the guest (visual only).

mod send;
mod show;
mod tick;

pub use tick::tick;

const ENEMY_BULLET_SUMMARY_INTERVAL_MS: u64 = 5000;
