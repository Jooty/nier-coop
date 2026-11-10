use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Once, OnceLock};

use replicant_game::log_stamp::{header_line, role_of_log_name, stamped_line, unix_millis_now};

use crate::net::{AutoConfig, LOG_ENV, resolve_log_path};

const BUILD_COMMIT: &str = env!("NC_BUILD_COMMIT");

static HEADER_WRITTEN: Once = Once::new();

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

pub fn stamp(message: &str) -> String {
    stamped_line(unix_millis_now(), message)
}

pub fn log_line(message: &str) {
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path())
    {
        HEADER_WRITTEN.call_once(|| {
            let header = header_line(
                unix_millis_now(),
                BUILD_COMMIT,
                role_of_log_name(log_path()),
            );
            let _ = writeln!(file, "{header}");
        });
        let _ = writeln!(file, "{}", stamp(message));
    }
}
