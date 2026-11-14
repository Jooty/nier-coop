use std::path::{Path, PathBuf};

use crate::saves::{BackupError, backup_saves, copy_dir_recursive};

pub const PRE_RESTORE_PREFIX: &str = "before-restore-";

pub fn list_backups(backup_root: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(backup_root) else {
        return Vec::new();
    };
    let mut stamps: Vec<String> = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            entry
                .file_type()
                .ok()?
                .is_dir()
                .then(|| entry.file_name().into_string().ok())
                .flatten()
        })
        .collect();
    stamps.sort();
    stamps.reverse();
    stamps
}

pub fn restore_backup(
    save_dir: &Path,
    backup_root: &Path,
    stamp: &str,
    now_stamp: &str,
) -> Result<PathBuf, BackupError> {
    let source = backup_root.join(stamp);
    if !source.is_dir() {
        return Err(BackupError::NoSaveFolder(source));
    }
    let safety_copy = if save_dir.is_dir() {
        let (path, _) = backup_saves(
            save_dir,
            backup_root,
            &format!("{PRE_RESTORE_PREFIX}{now_stamp}"),
        )?;
        Some(path)
    } else {
        None
    };
    copy_dir_recursive(&source, save_dir)?;
    Ok(safety_copy.unwrap_or(source))
}
