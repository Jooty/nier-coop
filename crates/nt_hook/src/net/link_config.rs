use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

pub const AUTO_CONFIG_FILE_NAME: &str = "nier-coop.toml";
pub const LINK_ENV: &str = "NIER_COOP_LINK";
pub const NAME_ENV: &str = "NIER_COOP_NAME";
pub const LOG_ENV: &str = "NIER_COOP_LOG";
pub const DEFAULT_LOG_FILE_NAME: &str = "nier-coop.log";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    Unreadable(String),
    Invalid(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreadable(message) => {
                write!(formatter, "cannot read {AUTO_CONFIG_FILE_NAME}: {message}")
            }
            Self::Invalid(message) => {
                write!(formatter, "{AUTO_CONFIG_FILE_NAME} is invalid: {message}")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

pub fn game_folder() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
}

pub fn read_config_text(folder: &Path) -> Result<Option<String>, ConfigError> {
    match fs::read_to_string(folder.join(AUTO_CONFIG_FILE_NAME)) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(ConfigError::Unreadable(error.to_string())),
    }
}

pub fn resolve_log_path(
    from_environment: Option<String>,
    from_config: impl FnOnce() -> Option<String>,
) -> String {
    from_environment
        .filter(|path| !path.trim().is_empty())
        .or_else(|| from_config().filter(|path| !path.trim().is_empty()))
        .unwrap_or_else(|| DEFAULT_LOG_FILE_NAME.to_owned())
}

pub fn parse_env_number<T: std::str::FromStr>(key: &str, text: &str) -> Result<T, ConfigError> {
    text.trim()
        .parse()
        .map_err(|_| ConfigError::Invalid(format!("{key} is not a number: {text}")))
}

pub fn parse_env_flag(key: &str, text: &str) -> Result<bool, ConfigError> {
    match text.trim().to_ascii_lowercase().as_str() {
        "1" | "true" => Ok(true),
        "0" | "false" => Ok(false),
        _ => Err(ConfigError::Invalid(format!(
            "{key} must be 1, 0, true or false: {text}"
        ))),
    }
}
