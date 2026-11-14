mod home;
mod migrate;
mod store;

pub use home::{ConfigHome, ConfigRole, open_config_home, process_config_home};
pub use migrate::{
    FolderMove, OLD_APP_DIR_NAMES, migrate_app_folders, move_folder, move_old_folders_into,
    plan_folder_move,
};
pub use store::{LauncherConfig, cache_dir, config_dir, config_path, data_dir};

pub const APP_DIR_NAME: &str = "niertogether";
pub const CONFIG_DIR_ENV: &str = "NIERTOGETHER_CONFIG_DIR";
pub const CONFIG_FILE_NAME: &str = "launcher.toml";
pub const LOCK_FILE_NAME: &str = "launcher.lock";
pub const SECOND_LAUNCHER_DIR_NAME: &str = "second";
pub const LOCK_RETRY_COUNT: u32 = 5;
pub const LOCK_RETRY_PAUSE_MS: u64 = 100;
