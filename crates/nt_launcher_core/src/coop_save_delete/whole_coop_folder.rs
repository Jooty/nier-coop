use std::path::{Path, PathBuf};

use crate::profile::game_by_id;

pub fn whole_coop_saves_folder(game_id: &str) -> Option<&'static str> {
    game_by_id(game_id).whole_coop_saves_folder()
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
