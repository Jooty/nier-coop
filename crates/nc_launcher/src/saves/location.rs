use std::path::PathBuf;

use crate::config::data_dir;
use crate::saves::{
    BACKUP_FOLDER, COOP_BACKUP_FOLDER, MY_GAMES_FOLDER, PROTON_SAVE_PREFIX_SEGMENTS,
};
use crate::steam::GameInstall;

const DOCUMENTS_FOLDER: &str = "Documents";

pub fn save_dir(game: &GameInstall) -> Option<PathBuf> {
    if !game.profile.support.saves {
        return None;
    }
    let documents = match &game.proton_prefix {
        Some(prefix) if !cfg!(windows) => PROTON_SAVE_PREFIX_SEGMENTS
            .iter()
            .fold(prefix.clone(), |path, segment| path.join(segment))
            .join(DOCUMENTS_FOLDER),
        _ => dirs::document_dir()?,
    };
    Some(
        documents
            .join(MY_GAMES_FOLDER)
            .join(game.profile.install_folder_name),
    )
}

pub fn backup_root() -> Option<PathBuf> {
    data_dir().map(|dir| dir.join(BACKUP_FOLDER))
}

pub fn coop_backup_root() -> Option<PathBuf> {
    data_dir().map(|dir| dir.join(COOP_BACKUP_FOLDER))
}
