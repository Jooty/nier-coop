use std::path::PathBuf;
use std::process::Command;

use crate::config::LauncherConfig;
use crate::steam::{
    GameInstall, candidate_steam_roots, find_game, install_from_game_dir, parse_reg_query_value,
};

const REGISTRY_KEY: &str = "HKCU\\Software\\Valve\\Steam";
const REGISTRY_VALUE: &str = "SteamPath";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn detect_game(config: &LauncherConfig) -> Option<GameInstall> {
    if let Some(game_dir) = config.game_dir.as_ref().filter(|dir| dir.is_dir()) {
        return Some(install_from_game_dir(game_dir));
    }
    let registry_path = read_registry_steam_path();
    let roots: Vec<PathBuf> =
        candidate_steam_roots(dirs::home_dir().as_deref(), registry_path.as_deref());
    find_game(&roots)
}

pub fn read_registry_steam_path() -> Option<String> {
    if !cfg!(windows) {
        return None;
    }
    let mut command = Command::new("reg");
    command.args(["query", REGISTRY_KEY, "/v", REGISTRY_VALUE]);
    hide_console_window(&mut command);
    let output = command.output().ok()?;
    parse_reg_query_value(&String::from_utf8_lossy(&output.stdout))
}

#[cfg(windows)]
pub fn hide_console_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
pub fn hide_console_window(_command: &mut Command) {
    let _ = CREATE_NO_WINDOW;
}
