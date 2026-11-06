use std::path::{Path, PathBuf};

use rc_launcher::second_copy::{copy_command, copy_environment, proton_script_from_config_info};
use rc_launcher::steam::{EXE_NAME, GameInstall};

fn install() -> GameInstall {
    GameInstall {
        game_dir: PathBuf::from("/lib/steamapps/common/NieR"),
        steamapps_dir: PathBuf::from("/lib/steamapps"),
        proton_prefix: None,
    }
}

fn value<'a>(envs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    envs.iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.as_str())
}

#[test]
fn the_second_copy_is_a_guest_with_its_own_link() {
    let envs = copy_environment(47300, "Guest", Some(2));
    assert_eq!(value(&envs, "REPLICANT_COOP_LINK"), Some("47300"));
    assert_eq!(value(&envs, "REPLICANT_COOP_NAME"), Some("Guest"));
    assert_eq!(value(&envs, "REPLICANT_COOP_BOOT_SLOT"), Some("2"));
    assert_eq!(value(&envs, "REPLICANT_COOP_BLOCK_SAVES"), Some("1"));
}

#[test]
fn no_slot_means_no_boot_slot_variable() {
    let envs = copy_environment(47300, "Guest", None);
    assert_eq!(value(&envs, "REPLICANT_COOP_BOOT_SLOT"), None);
}

#[test]
fn without_proton_the_exe_starts_directly() {
    let command = copy_command(&install(), copy_environment(1, "G", None), None);
    assert_eq!(command.program, install().exe_path());
    assert!(command.args.is_empty());
    assert_eq!(command.current_dir, install().game_dir);
    assert_eq!(value(&command.envs, "SteamAppId"), None);
}

#[test]
fn with_proton_the_exe_runs_through_the_proton_script() {
    let script = Path::new("/lib/steamapps/common/Proton 9.0/proton");
    let command = copy_command(&install(), copy_environment(1, "G", None), Some(script));
    assert_eq!(command.program, script);
    assert_eq!(command.args[0], "run");
    assert_eq!(Path::new(&command.args[1]), install().exe_path());
    assert!(command.args[1].to_string_lossy().ends_with(EXE_NAME));
    assert_eq!(
        value(&command.envs, "STEAM_COMPAT_DATA_PATH"),
        Some("/lib/steamapps/compatdata/1113560")
    );
    assert_eq!(
        value(&command.envs, "STEAM_COMPAT_CLIENT_INSTALL_PATH"),
        Some("/lib")
    );
    assert_eq!(value(&command.envs, "SteamAppId"), Some("1113560"));
    assert_eq!(value(&command.envs, "SteamGameId"), Some("1113560"));
    assert_eq!(value(&command.envs, "REPLICANT_COOP_LINK"), Some("1"));
}

#[test]
fn the_proton_script_sits_beside_the_files_folder() {
    let info = "9.0-4\n1700000000\n/home/u/steam/common/Proton 9.0/files/share/default_pfx/\n";
    assert_eq!(
        proton_script_from_config_info(info),
        Some(PathBuf::from("/home/u/steam/common/Proton 9.0/proton"))
    );
    assert_eq!(proton_script_from_config_info("nothing"), None);
}
