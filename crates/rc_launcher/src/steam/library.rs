use std::path::PathBuf;

use crate::steam::vdf::{VdfNode, parse_vdf};

pub fn library_paths(libraryfolders_vdf: &str) -> Vec<PathBuf> {
    let Ok(root) = parse_vdf(libraryfolders_vdf) else {
        return Vec::new();
    };
    let Some(folders) = root.get("libraryfolders") else {
        return Vec::new();
    };
    folders
        .entries()
        .iter()
        .filter(|(key, _)| !key.is_empty() && key.chars().all(|c| c.is_ascii_digit()))
        .filter_map(|(_, node)| match node {
            VdfNode::Text(path) => Some(path.as_str()),
            VdfNode::Map(_) => node.get("path").and_then(VdfNode::text),
        })
        .map(PathBuf::from)
        .collect()
}

pub fn manifest_install_dir(appmanifest_acf: &str) -> Option<String> {
    let root = parse_vdf(appmanifest_acf).ok()?;
    let name = root.get("AppState")?.get("installdir")?.text()?;
    (!name.is_empty()).then(|| name.to_owned())
}
