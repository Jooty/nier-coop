mod choice;
mod sync;

pub use choice::{paused_players_can_be_hit, set_paused_players_can_be_hit};
pub use sync::tick;

pub const PAUSED_HITS_RESEND_MS: u64 = 2_000;
