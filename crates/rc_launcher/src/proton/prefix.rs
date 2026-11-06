use std::path::{Path, PathBuf};

const FILES_SEGMENT: &str = "/files/";

pub fn find_proton_files_dir(config_info: &str) -> Option<PathBuf> {
    config_info
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('/'))
        .find_map(|line| {
            let end = line.rfind(FILES_SEGMENT)? + FILES_SEGMENT.len();
            Some(PathBuf::from(&line[..end]))
        })
}

pub fn environ_uses_prefix(environ: &[u8], prefix: &Path) -> bool {
    let wanted = format!("WINEPREFIX={}", prefix.display());
    environ
        .split(|byte| *byte == 0)
        .any(|entry| entry == wanted.as_bytes())
}

pub fn prefix_is_busy(prefix: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return false;
    };
    entries.filter_map(Result::ok).any(|entry| {
        std::fs::read(entry.path().join("environ"))
            .map(|bytes| environ_uses_prefix(&bytes, prefix))
            .unwrap_or(false)
    })
}
