use crate::log_upload::NO_ROLE_NAME;

const GAME_LOG_PREFIX: &str = "game-";
const RUN_LOG_PREFIXES: [&str; 4] = [GAME_LOG_PREFIX, "launcher", "second-copy", "crash"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogFileEntry {
    pub name: String,
    pub modified_secs: u64,
}

pub fn is_run_log_name(name: &str) -> bool {
    RUN_LOG_PREFIXES
        .iter()
        .any(|prefix| name.starts_with(prefix))
}

pub fn pick_since(entries: &[LogFileEntry], since_secs: u64) -> Vec<String> {
    entries
        .iter()
        .filter(|entry| is_run_log_name(&entry.name) && entry.modified_secs >= since_secs)
        .map(|entry| entry.name.clone())
        .collect()
}

pub fn pick_unsent(
    entries: &[LogFileEntry],
    sent_marker_secs: Option<u64>,
    launcher_started_secs: u64,
) -> Vec<String> {
    let picked: Vec<String> = entries
        .iter()
        .filter(|entry| {
            is_run_log_name(&entry.name)
                && entry.modified_secs > sent_marker_secs.unwrap_or(0)
                && entry.modified_secs < launcher_started_secs
        })
        .map(|entry| entry.name.clone())
        .collect();
    if picked.iter().any(|name| name.starts_with(GAME_LOG_PREFIX)) {
        picked
    } else {
        Vec::new()
    }
}

pub fn role_of_files(names: &[String]) -> &'static str {
    names
        .iter()
        .find_map(|name| {
            name.strip_prefix(GAME_LOG_PREFIX)
                .and_then(|rest| rest.split(['.', '-']).next())
                .and_then(|role| match role {
                    "host" => Some("host"),
                    "guest" => Some("guest"),
                    _ => None,
                })
        })
        .unwrap_or(NO_ROLE_NAME)
}
