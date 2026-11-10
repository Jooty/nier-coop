use std::path::{Path, PathBuf};

use nc_launcher_core::profile_by_id;

use crate::update::{
    AvailableUpdate, OLD_HOOK_DLL_SUFFIX, STAGING_DIR_NAME, UpdateError, download_verified,
};

pub fn apply_update(update: &AvailableUpdate, current_exe: &Path) -> Result<(), UpdateError> {
    let exe_dir = current_exe
        .parent()
        .ok_or_else(|| UpdateError::Io("the launcher has no folder".to_owned()))?;
    let staging = exe_dir.join(STAGING_DIR_NAME);
    let result = replace_from_staging(update, exe_dir, &staging);
    let _ = std::fs::remove_dir_all(&staging);
    result
}

pub fn apply_hook_update(update: &AvailableUpdate, current_exe: &Path) -> Result<(), UpdateError> {
    let exe_dir = current_exe
        .parent()
        .ok_or_else(|| UpdateError::Io("the launcher has no folder".to_owned()))?;
    let staging = exe_dir.join(STAGING_DIR_NAME);
    let result = replace_hook_from_staging(update, exe_dir, &staging);
    let _ = std::fs::remove_dir_all(&staging);
    result
}

pub fn swap_in_hook_dll(staged: &Path, destination: &Path) -> Result<Option<PathBuf>, UpdateError> {
    let backup = (destination.is_file()).then(|| old_path_of(destination));
    if let Some(backup) = &backup {
        std::fs::rename(destination, backup)?;
    }
    if let Err(error) = std::fs::rename(staged, destination) {
        if let Some(backup) = &backup {
            let _ = std::fs::rename(backup, destination);
        }
        return Err(error.into());
    }
    Ok(backup)
}

pub fn restore_hook_dll(backup: &Path, destination: &Path) {
    let _ = std::fs::remove_file(destination);
    let _ = std::fs::rename(backup, destination);
}

fn replace_from_staging(
    update: &AvailableUpdate,
    exe_dir: &Path,
    staging: &Path,
) -> Result<(), UpdateError> {
    let _ = std::fs::remove_dir_all(staging);
    std::fs::create_dir_all(staging)?;
    let staged_hooks = download_hooks(update, staging)?;
    let staged_launcher = staging.join(&update.plan.launcher.name);
    download_verified(
        &update.file_url(&update.plan.launcher.name),
        &update.plan.launcher,
        &staged_launcher,
    )?;
    make_executable(&staged_launcher)?;
    let swapped = swap_in_hooks(&staged_hooks, exe_dir)?;
    if let Err(error) = self_replace::self_replace(&staged_launcher) {
        undo_swaps(&swapped);
        return Err(UpdateError::Io(error.to_string()));
    }
    drop_backups(swapped);
    Ok(())
}

fn replace_hook_from_staging(
    update: &AvailableUpdate,
    exe_dir: &Path,
    staging: &Path,
) -> Result<(), UpdateError> {
    let _ = std::fs::remove_dir_all(staging);
    std::fs::create_dir_all(staging)?;
    let staged_hooks = download_hooks(update, staging)?;
    let swapped = swap_in_hooks(&staged_hooks, exe_dir)?;
    drop_backups(swapped);
    Ok(())
}

fn download_hooks(
    update: &AvailableUpdate,
    staging: &Path,
) -> Result<Vec<(&'static str, PathBuf)>, UpdateError> {
    let mut staged = Vec::new();
    for hook in &update.plan.hooks {
        let Some(profile) = profile_by_id(&hook.game_id) else {
            continue;
        };
        let staged_path = staging.join(&hook.file.name);
        download_verified(&update.file_url(&hook.file.name), &hook.file, &staged_path)?;
        staged.push((profile.hook_dll_name, staged_path));
    }
    if staged.is_empty() {
        return Err(UpdateError::MissingFile(
            "hook dll for a game this launcher knows".to_owned(),
        ));
    }
    Ok(staged)
}

fn swap_in_hooks(
    staged: &[(&'static str, PathBuf)],
    exe_dir: &Path,
) -> Result<Vec<(PathBuf, Option<PathBuf>)>, UpdateError> {
    let mut swapped: Vec<(PathBuf, Option<PathBuf>)> = Vec::new();
    for (dll_name, staged_path) in staged {
        let destination = exe_dir.join(dll_name);
        match swap_in_hook_dll(staged_path, &destination) {
            Ok(backup) => swapped.push((destination, backup)),
            Err(error) => {
                undo_swaps(&swapped);
                return Err(error);
            }
        }
    }
    Ok(swapped)
}

fn undo_swaps(swapped: &[(PathBuf, Option<PathBuf>)]) {
    for (destination, backup) in swapped {
        match backup {
            Some(backup) => restore_hook_dll(backup, destination),
            None => {
                let _ = std::fs::remove_file(destination);
            }
        }
    }
}

fn drop_backups(swapped: Vec<(PathBuf, Option<PathBuf>)>) {
    for (_, backup) in swapped {
        if let Some(backup) = backup {
            let _ = std::fs::remove_file(backup);
        }
    }
}

fn old_path_of(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(OLD_HOOK_DLL_SUFFIX);
    PathBuf::from(name)
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<(), UpdateError> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    Ok(())
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<(), UpdateError> {
    Ok(())
}
