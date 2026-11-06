mod file;

pub use file::{GAME_LINK_FILE_NAME, clear_boot, clear_link, write_boot, write_link};

pub const LINK_KEY: &str = "link";
pub const NAME_KEY: &str = "name";
pub const BOOT_SLOT_KEY: &str = "boot_slot";
pub const BLOCK_SAVES_KEY: &str = "block_saves";
