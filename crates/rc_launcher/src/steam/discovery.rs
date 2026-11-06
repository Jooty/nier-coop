use std::path::{Path, PathBuf};

use crate::steam::library::{library_paths, manifest_install_dir};
use crate::steam::{APP_ID, EXE_NAME};

const LIBRARY_FOLDERS_FILE: &str = "libraryfolders.vdf";
const REG_VALUE_TYPE: &str = "REG_SZ";
const WINDOWS_DEFAULT_STEAM_PATHS: [&str; 2] =
    ["C:\\Program Files (x86)\\Steam", "C:\\Program Files\\Steam"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameInstall {
    pub game_dir: PathBuf,
    pub steamapps_dir: PathBuf,
    pub proton_prefix: Option<PathBuf>,
}

impl GameInstall {
    pub fn exe_path(&self) -> PathBuf {
        self.game_dir.join(EXE_NAME)
    }

    pub fn exe_present(&self) -> bool {
        self.exe_path().is_file()
    }

    pub fn proton_compat_dir(&self) -> PathBuf {
        self.steamapps_dir.join("compatdata").join(APP_ID)
    }
}

pub fn candidate_steam_roots(
    home: Option<&Path>,
    registry_steam_path: Option<&str>,
) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(path) = registry_steam_path {
        roots.push(PathBuf::from(path));
    }
    if cfg!(windows) {
        roots.extend(WINDOWS_DEFAULT_STEAM_PATHS.iter().map(PathBuf::from));
    }
    if let Some(home) = home {
        roots.push(home.join(".steam/steam"));
        roots.push(home.join(".local/share/Steam"));
        roots.push(home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"));
        roots.push(home.join(".var/app/com.valvesoftware.Steam/.steam/steam"));
        roots.push(home.join("snap/steam/common/.local/share/Steam"));
        roots.push(home.join("snap/steam/common/.steam/steam"));
    }
    roots
}

pub fn parse_reg_query_value(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let (_, rest) = line.split_once(REG_VALUE_TYPE)?;
        let value = rest.trim();
        (!value.is_empty()).then(|| value.to_owned())
    })
}

pub fn find_game(steam_roots: &[PathBuf]) -> Option<GameInstall> {
    let mut libraries: Vec<PathBuf> = Vec::new();
    for root in steam_roots {
        push_unique(&mut libraries, root.clone());
        let listing = [
            root.join("steamapps").join(LIBRARY_FOLDERS_FILE),
            root.join("config").join(LIBRARY_FOLDERS_FILE),
        ]
        .iter()
        .find_map(|path| std::fs::read_to_string(path).ok());
        for library in listing.iter().flat_map(|text| library_paths(text)) {
            push_unique(&mut libraries, library);
        }
    }
    libraries
        .iter()
        .find_map(|library| install_in_library(library))
}

pub fn install_from_game_dir(game_dir: &Path) -> GameInstall {
    let steamapps_dir = game_dir
        .parent()
        .and_then(Path::parent)
        .map_or_else(|| game_dir.to_path_buf(), Path::to_path_buf);
    GameInstall {
        proton_prefix: existing_prefix(&steamapps_dir),
        game_dir: game_dir.to_path_buf(),
        steamapps_dir,
    }
}

fn install_in_library(library: &Path) -> Option<GameInstall> {
    let steamapps_dir = library.join("steamapps");
    let manifest =
        std::fs::read_to_string(steamapps_dir.join(format!("appmanifest_{APP_ID}.acf"))).ok()?;
    let install_dir = manifest_install_dir(&manifest)?;
    let game_dir = steamapps_dir.join("common").join(install_dir);
    game_dir.is_dir().then(|| GameInstall {
        proton_prefix: existing_prefix(&steamapps_dir),
        game_dir,
        steamapps_dir,
    })
}

fn existing_prefix(steamapps_dir: &Path) -> Option<PathBuf> {
    let prefix = steamapps_dir.join("compatdata").join(APP_ID).join("pfx");
    prefix.is_dir().then_some(prefix)
}

fn push_unique(list: &mut Vec<PathBuf>, path: PathBuf) {
    if !list.contains(&path) {
        list.push(path);
    }
}
