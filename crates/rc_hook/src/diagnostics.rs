mod log_file;

pub use log_file::log_line;
#[cfg(windows)]
pub use log_file::{log_path, stamp};
