use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde::Deserialize;

pub const AUTO_CONFIG_FILE_NAME: &str = "replicant-coop.toml";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutoConfig {
    pub link: Option<u16>,
    #[serde(default)]
    pub harness: bool,
    pub name: Option<String>,
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
        Ok(config)
    }

    pub fn load_from_folder(folder: &Path) -> Result<Option<Self>, AutoConfigError> {
        match fs::read_to_string(folder.join(AUTO_CONFIG_FILE_NAME)) {
            Ok(text) => Self::parse(&text).map(Some),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(AutoConfigError::Unreadable(error.to_string())),
        }
    }
}
