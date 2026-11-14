mod detour;
mod fight;
mod state;
mod tick;

pub use detour::install;
#[cfg(feature = "probes")]
pub use fight::is_boss_fight_on;
pub use fight::{is_boss, observe as observe_boss_fight};
pub use state::boss_message_counts;
pub use tick::tick;
