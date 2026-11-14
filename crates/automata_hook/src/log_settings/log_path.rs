use std::path::Path;

use crate::log_settings::DEFAULT_LOG_FILE_NAME;

pub fn resolve_log_path(
    from_environment: Option<String>,
    from_config: impl FnOnce() -> Option<String>,
) -> String {
    from_environment
        .filter(|path| !path.trim().is_empty())
        .or_else(|| from_config().filter(|path| !path.trim().is_empty()))
        .unwrap_or_else(|| DEFAULT_LOG_FILE_NAME.to_owned())
}

/// The game changes its current folder while it starts, so a relative path is tied to the
/// exe's folder once.
pub fn absolute_log_path(path: String, exe_folder: Option<&Path>) -> String {
    match exe_folder {
        Some(folder) if Path::new(&path).is_relative() => {
            folder.join(&path).to_string_lossy().into_owned()
        }
        _ => path,
    }
}
