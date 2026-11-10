mod migrate;
mod store;

pub use migrate::{
    FolderMove, OLD_APP_DIR_NAME, migrate_app_folders, move_folder, plan_folder_move,
};
pub use store::{LauncherConfig, cache_dir, config_dir, config_dir_from, config_path, data_dir};

pub const APP_DIR_NAME: &str = "nier-coop";
pub const CONFIG_DIR_ENV: &str = "NIER_COOP_CONFIG_DIR";
pub const CONFIG_FILE_NAME: &str = "launcher.toml";
