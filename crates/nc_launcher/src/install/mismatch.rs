use std::path::Path;

use crate::install::{PROXY_DLL_NAME, sha256_file};

const SHORT_HASH_CHARS: usize = 8;

pub fn mod_mismatch(game_dir: &Path, source_dll: Option<&Path>) -> Option<String> {
    let installed = game_dir.join(PROXY_DLL_NAME);
    let Some(source) = source_dll else {
        return Some("no mod DLL to compare the installed one with".to_owned());
    };
    let installed_hash = sha256_file(&installed).ok();
    let source_hash = sha256_file(source).ok();
    if installed_hash.is_some() && installed_hash == source_hash {
        return None;
    }
    let short = |hash: &Option<String>| {
        hash.as_ref().map_or_else(
            || "missing".to_owned(),
            |hash| hash.chars().take(SHORT_HASH_CHARS).collect(),
        )
    };
    Some(format!(
        "{} in the game folder ({}) differs from the mod DLL {} ({}); close the game and install the mod before starting a second copy",
        PROXY_DLL_NAME,
        short(&installed_hash),
        source.display(),
        short(&source_hash)
    ))
}
