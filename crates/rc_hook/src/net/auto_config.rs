use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde::Deserialize;

pub const AUTO_CONFIG_FILE_NAME: &str = "replicant-coop.toml";
pub const LINK_ENV: &str = "REPLICANT_COOP_LINK";
pub const NAME_ENV: &str = "REPLICANT_COOP_NAME";
pub const BOOT_SLOT_ENV: &str = "REPLICANT_COOP_BOOT_SLOT";
pub const BLOCK_SAVES_ENV: &str = "REPLICANT_COOP_BLOCK_SAVES";
pub const LOG_ENV: &str = "REPLICANT_COOP_LOG";
const ALL_ENV_KEYS: [&str; 5] = [LINK_ENV, NAME_ENV, BOOT_SLOT_ENV, BLOCK_SAVES_ENV, LOG_ENV];
pub const DEFAULT_LOG_FILE_NAME: &str = "replicant-coop.log";
const SAVE_SLOT_COUNT: u8 = 32;

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutoConfig {
    pub link: Option<u16>,
    pub name: Option<String>,
    pub boot_slot: Option<u8>,
    #[serde(default)]
    pub block_saves: bool,
    #[serde(default)]
    pub keep_running: bool,
    #[serde(default)]
    pub dev_no_focus_menu: bool,
    pub log: Option<String>,
    #[serde(default)]
    pub show_distance: bool,
    #[serde(default)]
    pub show_ping: bool,
    pub event_log: Option<bool>,
    pub show_names: Option<bool>,
    pub show_party_frame: Option<bool>,
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
        toml::from_str::<Self>(text)
            .map_err(|error| AutoConfigError::Invalid(error.to_string()))?
            .validated()
    }

    fn validated(self) -> Result<Self, AutoConfigError> {
        if self.link == Some(0) {
            return Err(AutoConfigError::Invalid("link must not be 0".to_owned()));
        }
        if self.boot_slot.is_some_and(|slot| slot >= SAVE_SLOT_COUNT) {
            return Err(AutoConfigError::Invalid(format!(
                "boot_slot must be 0 to {}",
                SAVE_SLOT_COUNT - 1
            )));
        }
        Ok(self)
    }

    pub fn event_log_enabled(&self) -> bool {
        self.event_log.unwrap_or(true)
    }

    pub fn load_beside_game() -> Result<Option<Self>, AutoConfigError> {
        let folder = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf));
        match folder {
            Some(folder) => Self::load_layered(&folder, |key| std::env::var(key).ok()),
            None => Self::layered(None, |key| std::env::var(key).ok()),
        }
    }

    pub fn load_layered(
        folder: &Path,
        environment: impl Fn(&str) -> Option<String>,
    ) -> Result<Option<Self>, AutoConfigError> {
        Self::layered(Self::load_from_folder(folder)?, environment)
    }

    pub fn layered(
        file: Option<Self>,
        environment: impl Fn(&str) -> Option<String>,
    ) -> Result<Option<Self>, AutoConfigError> {
        if file.is_none() && ALL_ENV_KEYS.iter().all(|key| environment(key).is_none()) {
            return Ok(None);
        }
        let mut config = file.unwrap_or_default();
        if let Some(text) = environment(LINK_ENV) {
            config.link = Some(parse_env_number(LINK_ENV, &text)?);
        }
        if let Some(text) = environment(NAME_ENV) {
            config.name = Some(text);
        }
        if let Some(text) = environment(BOOT_SLOT_ENV) {
            config.boot_slot = Some(parse_env_number(BOOT_SLOT_ENV, &text)?);
        }
        if let Some(text) = environment(BLOCK_SAVES_ENV) {
            config.block_saves = parse_env_flag(&text)?;
        }
        if let Some(text) = environment(LOG_ENV) {
            config.log = Some(text);
        }
        config.validated().map(Some)
    }

    pub fn load_from_folder(folder: &Path) -> Result<Option<Self>, AutoConfigError> {
        match fs::read_to_string(folder.join(AUTO_CONFIG_FILE_NAME)) {
            Ok(text) => Self::parse(&text).map(Some),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(AutoConfigError::Unreadable(error.to_string())),
        }
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

fn parse_env_number<T: std::str::FromStr>(key: &str, text: &str) -> Result<T, AutoConfigError> {
    text.trim()
        .parse()
        .map_err(|_| AutoConfigError::Invalid(format!("{key} is not a number: {text}")))
}

fn parse_env_flag(text: &str) -> Result<bool, AutoConfigError> {
    match text.trim().to_ascii_lowercase().as_str() {
        "1" | "true" => Ok(true),
        "0" | "false" => Ok(false),
        _ => Err(AutoConfigError::Invalid(format!(
            "{BLOCK_SAVES_ENV} must be 1, 0, true or false: {text}"
        ))),
    }
}
