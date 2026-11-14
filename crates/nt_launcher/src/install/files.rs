use std::path::Path;

use nt_launcher_core::GameProfile;
use thiserror::Error;

use crate::install::{Ledger, sha256_file};

#[derive(Debug, Error)]
pub enum InstallError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error(
        "a file with the loader name in the game folder is not one this launcher installed; move it away first"
    )]
    ForeignFileInTheWay,
    #[error("the loader name is one of the game's own files; refusing to touch it")]
    LoaderIsAGameFile,
    #[error("the copied file does not match the source")]
    CopyMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallOutcome {
    Installed,
    AlreadyCurrent,
    Updated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModStatus {
    NotInstalled,
    Current,
    Outdated,
    Foreign,
}

pub fn mod_status(
    profile: &GameProfile,
    game_dir: &Path,
    source_dll: Option<&Path>,
    ledger: &Ledger,
) -> ModStatus {
    let target = game_dir.join(profile.loader_dll_name);
    if !target.is_file() || profile.is_game_exe(profile.loader_dll_name) {
        return ModStatus::NotInstalled;
    }
    let Ok(installed_hash) = sha256_file(&target) else {
        return ModStatus::Foreign;
    };
    let source_hash = source_dll.and_then(|path| sha256_file(path).ok());
    if source_hash.as_deref() == Some(installed_hash.as_str()) {
        ModStatus::Current
    } else if ledger.contains(&installed_hash) {
        ModStatus::Outdated
    } else {
        ModStatus::Foreign
    }
}

pub fn install_dll(
    profile: &GameProfile,
    game_dir: &Path,
    source_dll: &Path,
    ledger: &Ledger,
) -> Result<InstallOutcome, InstallError> {
    if profile.is_game_exe(profile.loader_dll_name) {
        return Err(InstallError::LoaderIsAGameFile);
    }
    let target = game_dir.join(profile.loader_dll_name);
    let source_hash = sha256_file(source_dll)?;
    let existed = target.is_file();
    if existed {
        let installed_hash = sha256_file(&target)?;
        if installed_hash == source_hash {
            ledger.record(&source_hash)?;
            return Ok(InstallOutcome::AlreadyCurrent);
        }
        if !ledger.contains(&installed_hash) {
            return Err(InstallError::ForeignFileInTheWay);
        }
    }
    std::fs::copy(source_dll, &target)?;
    if sha256_file(&target)? != source_hash {
        return Err(InstallError::CopyMismatch);
    }
    ledger.record(&source_hash)?;
    Ok(if existed {
        InstallOutcome::Updated
    } else {
        InstallOutcome::Installed
    })
}

pub fn uninstall_dll(
    profile: &GameProfile,
    game_dir: &Path,
    ledger: &Ledger,
) -> Result<bool, InstallError> {
    if profile.is_game_exe(profile.loader_dll_name) {
        return Err(InstallError::LoaderIsAGameFile);
    }
    let target = game_dir.join(profile.loader_dll_name);
    if !target.is_file() {
        return Ok(false);
    }
    if !ledger.contains(&sha256_file(&target)?) {
        return Err(InstallError::ForeignFileInTheWay);
    }
    std::fs::remove_file(&target)?;
    Ok(true)
}
