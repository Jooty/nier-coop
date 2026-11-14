use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Once, OnceLock};

use crate::log_stamp::{header_line, role_of_log_name, stamped_line, unix_millis_now};

const FALLBACK_LOG_PATH: &str = "niertogether.log";
const UNKNOWN_BUILD_COMMIT: &str = "unknown";

static HEADER_WRITTEN: Once = Once::new();
static SETTINGS: OnceLock<LogSettings> = OnceLock::new();
static LOG_PATH: OnceLock<String> = OnceLock::new();

#[derive(Debug, Clone, Copy)]
pub struct LogSettings {
    pub build_commit: &'static str,
    pub resolve_path: fn() -> String,
}

pub fn configure(settings: LogSettings) {
    let _ = SETTINGS.set(settings);
}

pub fn log_path() -> &'static str {
    LOG_PATH.get_or_init(|| match SETTINGS.get() {
        Some(settings) => (settings.resolve_path)(),
        None => FALLBACK_LOG_PATH.to_owned(),
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
                SETTINGS
                    .get()
                    .map_or(UNKNOWN_BUILD_COMMIT, |settings| settings.build_commit),
                role_of_log_name(log_path()),
            );
            let _ = writeln!(file, "{header}");
        });
        let _ = writeln!(file, "{}", stamp(message));
    }
}
