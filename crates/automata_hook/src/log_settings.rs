mod config_log;
mod log_path;

pub use config_log::{log_from_config_beside_game, log_from_config_text};
pub use log_path::resolve_log_path;

pub const LOG_ENV: &str = "NIER_COOP_LOG";
pub const DEFAULT_LOG_FILE_NAME: &str = "nier-coop.log";
pub const CONFIG_FILE_NAME: &str = "nier-coop.toml";
