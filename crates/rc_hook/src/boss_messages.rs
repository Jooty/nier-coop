mod detour;
mod life_callbacks;
mod state;
mod tick;

pub use detour::install;
pub use life_callbacks::group_has_registered_life_callback;
pub use state::boss_message_counts;
pub use tick::tick;
