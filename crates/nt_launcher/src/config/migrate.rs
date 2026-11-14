use std::path::Path;

use crate::config::APP_DIR_NAME;

pub const OLD_APP_DIR_NAMES: [&str; 2] = ["nier-coop", "replicant-coop"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderMove {
    Nothing,
    RenameOldToNew,
}

pub fn plan_folder_move(old_exists: bool, new_exists: bool) -> FolderMove {
    if old_exists && !new_exists {
        FolderMove::RenameOldToNew
    } else {
        FolderMove::Nothing
    }
}

pub fn move_folder(old: &Path, new: &Path) -> std::io::Result<FolderMove> {
    let old_is_folder = std::fs::symlink_metadata(old).is_ok_and(|meta| meta.is_dir());
    let new_exists = std::fs::symlink_metadata(new).is_ok();
    let plan = plan_folder_move(old_is_folder, new_exists);
    if plan == FolderMove::RenameOldToNew {
        std::fs::rename(old, new)?;
        #[cfg(unix)]
        std::os::unix::fs::symlink(new, old)?;
    }
    Ok(plan)
}

pub fn move_old_folders_into(
    base: &Path,
    old_names: &[&str],
    new_name: &str,
) -> std::io::Result<FolderMove> {
    let new = base.join(new_name);
    let mut result = FolderMove::Nothing;
    for old_name in old_names {
        if move_folder(&base.join(old_name), &new)? == FolderMove::RenameOldToNew {
            result = FolderMove::RenameOldToNew;
        }
    }
    Ok(result)
}

pub fn migrate_app_folders() {
    let bases = [dirs::config_dir(), dirs::cache_dir(), dirs::data_dir()];
    for base in bases.into_iter().flatten() {
        if let Err(error) = move_old_folders_into(&base, &OLD_APP_DIR_NAMES, APP_DIR_NAME) {
            eprintln!(
                "could not move an old app folder in {} to {APP_DIR_NAME}: {error}",
                base.display()
            );
        }
    }
}
