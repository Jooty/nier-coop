use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::proton::find_proton_files_dir;
use crate::second_copy::{BLOCK_SAVES_ENV, BOOT_SLOT_ENV, LINK_ENV, NAME_ENV, PROTON_SCRIPT_NAME};
use crate::steam::{APP_ID, GameInstall};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyCommand {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub envs: Vec<(String, String)>,
    pub current_dir: PathBuf,
}

pub fn copy_environment(
    link_port: u16,
    name: &str,
    boot_slot: Option<u8>,
) -> Vec<(String, String)> {
    let mut envs = vec![
        (LINK_ENV.to_owned(), link_port.to_string()),
        (NAME_ENV.to_owned(), name.to_owned()),
        (BLOCK_SAVES_ENV.to_owned(), "1".to_owned()),
    ];
    if let Some(slot) = boot_slot {
        envs.push((BOOT_SLOT_ENV.to_owned(), slot.to_string()));
    }
    envs
}

pub fn proton_script_from_config_info(config_info: &str) -> Option<PathBuf> {
    let files_dir = find_proton_files_dir(config_info)?;
    Some(files_dir.parent()?.join(PROTON_SCRIPT_NAME))
}

pub fn copy_command(
    game: &GameInstall,
    copy_envs: Vec<(String, String)>,
    proton_script: Option<&Path>,
) -> CopyCommand {
    let exe = game.exe_path();
    let mut envs = copy_envs;
    let Some(script) = proton_script else {
        return CopyCommand {
            program: exe,
            args: Vec::new(),
            envs,
            current_dir: game.game_dir.clone(),
        };
    };
    let steam_root = game.steamapps_dir.parent().unwrap_or(&game.steamapps_dir);
    envs.extend([
        (
            "STEAM_COMPAT_DATA_PATH".to_owned(),
            game.proton_compat_dir().display().to_string(),
        ),
        (
            "STEAM_COMPAT_CLIENT_INSTALL_PATH".to_owned(),
            steam_root.display().to_string(),
        ),
        ("SteamAppId".to_owned(), APP_ID.to_owned()),
        ("SteamGameId".to_owned(), APP_ID.to_owned()),
    ]);
    CopyCommand {
        program: script.to_path_buf(),
        args: vec![OsString::from("run"), exe.into_os_string()],
        envs,
        current_dir: game.game_dir.clone(),
    }
}
