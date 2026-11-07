mod redirect;
mod reload;

pub use redirect::coop_save_name;
pub use reload::{ReloadProgress, reload_progress};

pub const GAMEDATA_FILE_NAME: &str = "GAMEDATA";
pub const COOP_GAMEDATA_FILE_NAME: &str = "GAMEDATA.coop";
pub const COOP_GAMEDATA_BACKUP_FILE_NAME: &str = "GAMEDATA.coop.bak";
