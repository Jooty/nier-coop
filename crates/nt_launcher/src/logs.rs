mod location;
mod names;
mod rotate;
mod upload;
mod wine_path;
mod writer;

pub use location::logs_dir;
pub use names::{game_log_name, launcher_log_name};
pub use rotate::rotate_log;
pub use upload::{
    UploadJob, UploadOutcome, gzip_bundle_of, list_log_entries, put_logs, read_sent_marker,
    run_upload, unix_seconds, write_sent_marker,
};
pub use wine_path::path_as_the_game_sees_it;
pub use writer::LogWriter;

pub const LOG_FILES_KEPT: usize = 5;
pub const LAUNCHER_LOG_BEFORE_ROLE: &str = "launcher.log";
pub const SENT_MARKER_FILE_NAME: &str = "logs-sent.txt";
