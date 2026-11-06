mod location;
mod names;
mod rotate;
mod wine_path;
mod writer;

pub use location::logs_dir;
pub use names::{game_log_name, launcher_log_name};
pub use rotate::rotate_log;
pub use wine_path::path_as_the_game_sees_it;
pub use writer::LogWriter;

pub const LOG_FILES_KEPT: usize = 5;
pub const LAUNCHER_LOG_BEFORE_ROLE: &str = "launcher.log";
