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
