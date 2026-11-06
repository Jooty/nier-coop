use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde::Deserialize;

pub const AUTO_CONFIG_FILE_NAME: &str = "replicant-coop.toml";
const SAVE_SLOT_COUNT: u8 = 32;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutoConfig {
    pub link: Option<u16>,
    pub name: Option<String>,
    pub boot_slot: Option<u8>,
    #[serde(default)]
    pub block_saves: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutoConfigError {
    Unreadable(String),
    Invalid(String),
}

impl std::fmt::Display for AutoConfigError {
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

impl std::error::Error for AutoConfigError {}

impl AutoConfig {
    pub fn parse(text: &str) -> Result<Self, AutoConfigError> {
        let config: Self =
            toml::from_str(text).map_err(|error| AutoConfigError::Invalid(error.to_string()))?;
        if config.link == Some(0) {
            return Err(AutoConfigError::Invalid("link must not be 0".to_owned()));
        }
        if config.boot_slot.is_some_and(|slot| slot >= SAVE_SLOT_COUNT) {
            return Err(AutoConfigError::Invalid(format!(
                "boot_slot must be 0 to {}",
                SAVE_SLOT_COUNT - 1
            )));
        }
        Ok(config)
    }

    pub fn load_beside_game() -> Result<Option<Self>, AutoConfigError> {
        let folder = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf));
        match folder {
            Some(folder) => Self::load_from_folder(&folder),
            None => Ok(None),
        }
    }

    pub fn load_from_folder(folder: &Path) -> Result<Option<Self>, AutoConfigError> {
        match fs::read_to_string(folder.join(AUTO_CONFIG_FILE_NAME)) {
            Ok(text) => Self::parse(&text).map(Some),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(AutoConfigError::Unreadable(error.to_string())),
        }
    }
}
