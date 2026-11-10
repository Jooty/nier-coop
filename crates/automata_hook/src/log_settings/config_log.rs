use serde::Deserialize;

use crate::log_settings::CONFIG_FILE_NAME;

#[derive(Deserialize)]
struct LogKey {
    log: Option<String>,
}

pub fn log_from_config_text(text: &str) -> Option<String> {
    toml::from_str::<LogKey>(text).ok()?.log
}

pub fn log_from_config_beside_game() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let text = std::fs::read_to_string(exe.parent()?.join(CONFIG_FILE_NAME)).ok()?;
    log_from_config_text(&text)
}
