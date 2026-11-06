use std::fs::OpenOptions;
use std::io::Write;

use crate::diagnostics::recent_lines::remember_line;

const LOG_FILE_NAME: &str = "replicant-coop.log";

pub fn log_line(message: &str) {
    remember_line(message);
    // The game's working directory is its install folder under Steam and Proton.
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_FILE_NAME)
    {
        let _ = writeln!(file, "{message}");
    }
}
