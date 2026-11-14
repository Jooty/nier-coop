use std::path::{Path, PathBuf};

use crate::second_copy::{RUNTIME_ENTRY_POINT_NAME, STEAM_LAUNCHER_SCRIPT_NAME};
use crate::steam::manifest_install_dir;

const REQUIRE_TOOL_KEY: &str = "require_tool_appid";

pub fn required_tool_app_id(tool_manifest: &str) -> Option<String> {
    tool_manifest.lines().find_map(|line| {
        let mut quoted = line.split('"').skip(1).step_by(2);
        (quoted.next()? == REQUIRE_TOOL_KEY)
            .then(|| quoted.next().map(str::to_owned))
            .flatten()
            .filter(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()))
    })
}

pub fn runtime_entry_point(app_id: &str, steamapps_dirs: &[PathBuf]) -> Option<PathBuf> {
    steamapps_dirs.iter().find_map(|steamapps| {
        let manifest =
            std::fs::read_to_string(steamapps.join(format!("appmanifest_{app_id}.acf"))).ok()?;
        let entry = steamapps
            .join("common")
            .join(manifest_install_dir(&manifest)?)
            .join(RUNTIME_ENTRY_POINT_NAME);
        entry.is_file().then_some(entry)
    })
}

pub fn steam_client_root(config_info: &str, candidates: &[PathBuf]) -> Option<PathBuf> {
    let from_config_info = config_info
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('/') && !line.contains("/files/"))
        .map(PathBuf::from);
    candidates
        .iter()
        .cloned()
        .chain(from_config_info)
        .find(|root| has_steam_script(root))
}

fn has_steam_script(root: &Path) -> bool {
    root.join(STEAM_LAUNCHER_SCRIPT_NAME).is_file()
}
