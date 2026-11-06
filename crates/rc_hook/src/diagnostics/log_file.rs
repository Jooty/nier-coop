use std::fs::OpenOptions;
use std::io::Write;
use std::sync::OnceLock;

use crate::net::{AutoConfig, LOG_ENV, resolve_log_path};

pub fn log_line(message: &str) {
    static LOG_PATH: OnceLock<String> = OnceLock::new();
    let path = LOG_PATH.get_or_init(|| {
        resolve_log_path(std::env::var(LOG_ENV).ok(), || {
            AutoConfig::load_beside_game()
                .ok()
                .flatten()
                .and_then(|config| config.log)
        })
    });
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{message}");
    }
}
