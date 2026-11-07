mod calls;
mod send;
mod show;

pub(crate) use calls::{create_effect, end_effect, place_effect, style_effect};
pub use send::tick_own_bullets;
pub use show::tick_partner_bullets;

const SUMMARY_INTERVAL_MS: u64 = 5000;
