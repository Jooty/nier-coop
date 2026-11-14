use std::path::{Path, PathBuf};

use crate::coop_save_delete::WHOLE_COOP_SAVE_FOLDER_BY_GAME_ID;

pub fn whole_coop_saves_folder(game_id: &str) -> Option<&'static str> {
    WHOLE_COOP_SAVE_FOLDER_BY_GAME_ID
        .iter()
        .find(|(id, _)| *id == game_id)
        .map(|(_, folder)| *folder)
}

/// Everything directly inside a folder that holds only a game's co-op saves, sorted. A folder that
/// cannot be read holds none.
pub fn coop_saves_in_whole_folder(folder: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = entries
        .filter_map(|entry| Some(entry.ok()?.path()))
        .collect();
    found.sort();
    found
}
