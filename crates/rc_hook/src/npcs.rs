mod game_calls;
mod hold_hook;
mod sync;
mod talk_guard_hook;
mod talk_press;
mod talk_registry_hook;

pub use hold_hook::install;
pub use sync::tick_npc_sync;
