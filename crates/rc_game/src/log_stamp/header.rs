use std::time::{SystemTime, UNIX_EPOCH};

use crate::log_stamp::{date_time_text, time_prefix};

pub fn stamped_line(unix_millis: u64, message: &str) -> String {
    format!("{}{message}", time_prefix(unix_millis))
}

pub fn header_line(unix_millis: u64, commit: &str, role: &str) -> String {
    format!(
        "log opened {}, build {commit}, role {role}",
        date_time_text(unix_millis)
    )
}

pub fn role_of_log_name(name: &str) -> &'static str {
    let file = name.rsplit(['/', '\\']).next().unwrap_or(name);
    if file.contains("host") {
        "host"
    } else if file.contains("guest") {
        "guest"
    } else {
        "unknown"
    }
}

pub fn unix_millis_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
        })
}
