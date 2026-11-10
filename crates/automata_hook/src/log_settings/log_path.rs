use crate::log_settings::DEFAULT_LOG_FILE_NAME;

pub fn resolve_log_path(env_value: Option<String>) -> String {
    env_value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_LOG_FILE_NAME.to_owned())
}
