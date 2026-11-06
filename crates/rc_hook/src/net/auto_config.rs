use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde::Deserialize;

pub const AUTO_CONFIG_FILE_NAME: &str = "replicant-coop.toml";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AutoMode {
    Host,
    Join,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutoConfig {
    pub mode: AutoMode,
    pub ticket: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutoConfigError {
    Unreadable(String),
    Invalid(String),
    JoinWithoutTicket,
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
            Self::JoinWithoutTicket => {
                write!(
                    formatter,
                    "{AUTO_CONFIG_FILE_NAME}: mode \"join\" needs a ticket"
                )
            }
        }
    }
}

impl std::error::Error for AutoConfigError {}

impl AutoConfig {
    pub fn parse(text: &str) -> Result<Self, AutoConfigError> {
        let config: Self =
            toml::from_str(text).map_err(|error| AutoConfigError::Invalid(error.to_string()))?;
        let has_ticket = config
            .ticket
            .as_deref()
            .is_some_and(|ticket| !ticket.trim().is_empty());
        if config.mode == AutoMode::Join && !has_ticket {
            return Err(AutoConfigError::JoinWithoutTicket);
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
