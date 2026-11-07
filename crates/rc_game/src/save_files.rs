mod redirect;
mod reload;

pub use redirect::coop_save_name;
pub use reload::{ReloadInputs, ReloadProgress, reload_progress};

pub const GAMEDATA_FILE_NAME: &str = "GAMEDATA";
pub const COOP_GAMEDATA_FILE_NAME: &str = "GAMEDATA.coop";
pub const COOP_GAMEDATA_BACKUP_FILE_NAME: &str = "GAMEDATA.coop.bak";
pub const RELOAD_BUSY_GRACE_MS: u64 = 2_000;
pub const RELOAD_TIMEOUT_MS: u64 = 30_000;
