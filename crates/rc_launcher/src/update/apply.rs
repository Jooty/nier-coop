use std::path::{Path, PathBuf};

use crate::install::HOOK_DLL_NAME;
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
    let staged_hook = staging.join(&update.plan.hook.name);
    let staged_launcher = staging.join(&update.plan.launcher.name);
    download_verified(
        &update.file_url(&update.plan.hook.name),
        &update.plan.hook,
        &staged_hook,
    )?;
    download_verified(
        &update.file_url(&update.plan.launcher.name),
        &update.plan.launcher,
        &staged_launcher,
    )?;
    make_executable(&staged_launcher)?;
    let hook_destination = exe_dir.join(HOOK_DLL_NAME);
    let backup = swap_in_hook_dll(&staged_hook, &hook_destination)?;
    if let Err(error) = self_replace::self_replace(&staged_launcher) {
        match &backup {
            Some(backup) => restore_hook_dll(backup, &hook_destination),
            None => {
                let _ = std::fs::remove_file(&hook_destination);
            }
        }
        return Err(UpdateError::Io(error.to_string()));
    }
    if let Some(backup) = backup {
        let _ = std::fs::remove_file(backup);
    }
    Ok(())
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
