mod detour;
mod fight;
mod state;
mod tick;

pub use detour::install;
pub use fight::{is_boss, is_boss_fight_on, observe as observe_boss_fight};
pub use state::boss_message_counts;
pub use tick::tick;
