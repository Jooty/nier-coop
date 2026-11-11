mod automata_menu;
mod downed_hooks;
mod teleport_hooks;

pub use automata_menu::AutomataMenu;
pub use downed_hooks::DownedHooks;
pub use teleport_hooks::TeleportHooks;

pub const GIVE_UP_FIX_ID: u32 = 1;
pub const GIVE_UP_FIX_LABEL: &str = "Give up waiting for a revive (the game's own defeat runs)";
pub const NOT_DOWN_TEXT: &str = "the hero is not down";
