use serde::Deserialize;

use nt_hook::net::read_config_text;

#[derive(Deserialize)]
struct LogKey {
    log: Option<String>,
}

pub fn log_from_config_text(text: &str) -> Option<String> {
    toml::from_str::<LogKey>(text).ok()?.log
}

pub fn log_from_config_beside_game() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let text = read_config_text(exe.parent()?).ok()??;
    log_from_config_text(&text)
}
