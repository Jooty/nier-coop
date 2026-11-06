use std::fs::OpenOptions;
use std::io::Write;
use std::sync::OnceLock;

use crate::net::{AutoConfig, LOG_ENV, resolve_log_path};

pub fn log_path() -> &'static str {
    static LOG_PATH: OnceLock<String> = OnceLock::new();
    LOG_PATH.get_or_init(|| {
        resolve_log_path(std::env::var(LOG_ENV).ok(), || {
            AutoConfig::load_beside_game()
                .ok()
                .flatten()
                .and_then(|config| config.log)
        })
    })
}

pub fn log_line(message: &str) {
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path())
    {
        let _ = writeln!(file, "{message}");
    }
}
