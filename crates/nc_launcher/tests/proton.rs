use std::path::{Path, PathBuf};

use nc_launcher::proton::{
    environ_uses_prefix, exe_defaults_key, find_proton_files_dir, key_is_empty, override_is_set,
    override_key, override_keys,
};
use nc_launcher_core::{GameProfile, REPLICANT};

#[test]
fn proton_files_dir_is_taken_from_the_config_info_paths() {
    let info = "9.0-4\n1700000000\n/home/u/steam/common/Proton 9.0/files/share/default_pfx/\n";
    assert_eq!(
        find_proton_files_dir(info),
        Some(PathBuf::from("/home/u/steam/common/Proton 9.0/files/"))
    );
    assert_eq!(
        find_proton_files_dir("/opt/proton/files/share/x/"),
        Some(PathBuf::from("/opt/proton/files/"))
    );
    assert_eq!(find_proton_files_dir("no paths here"), None);
}

#[test]
fn a_busy_prefix_is_found_in_a_process_environment() {
    let prefix = Path::new("/compat/pfx");
    let busy = b"HOME=/home/u\0WINEPREFIX=/compat/pfx\0LANG=C\0";
    let other = b"WINEPREFIX=/compat/pfx2\0";
    assert!(environ_uses_prefix(busy, prefix));
    assert!(!environ_uses_prefix(other, prefix));
}

#[test]
fn override_key_targets_the_game_exe_and_detection_reads_wine_output() {
    assert!(
        override_key(REPLICANT.exe_names[0])
            .ends_with("NieR Replicant ver.1.22474487139.exe\\DllOverrides")
    );
    assert!(override_is_set(
        "    xinput9_1_0    REG_SZ    native,builtin"
    ));
    assert!(!override_is_set("    xinput9_1_0    REG_SZ    builtin"));
}

#[test]
fn a_game_with_two_exes_gets_an_override_key_for_each() {
    let two_exes = GameProfile {
        exe_names: &["Game.exe", "GameCompat.exe"],
        ..REPLICANT
    };
    let keys = override_keys(&two_exes);
    assert_eq!(keys.len(), 2);
    assert!(keys[0].ends_with("AppDefaults\\Game.exe\\DllOverrides"));
    assert!(keys[1].ends_with("AppDefaults\\GameCompat.exe\\DllOverrides"));
    assert_eq!(override_keys(&REPLICANT).len(), 1);
}

#[test]
fn a_key_is_empty_when_the_query_prints_only_its_own_line() {
    let key_line = "HKEY_CURRENT_USER\\Software\\Wine\\AppDefaults\\Made Up Game.exe\\DllOverrides";
    assert!(key_is_empty(&format!("\r\n{key_line}\r\n\r\n")));
    assert!(!key_is_empty(&format!(
        "{key_line}\r\n    made_up_loader    REG_SZ    native,builtin\r\n"
    )));
    assert!(!key_is_empty(&format!("{key_line}\r\n{key_line}\\Sub\r\n")));
    assert!(!key_is_empty(""));
}

#[test]
fn the_parent_of_an_override_key_is_the_exes_own_defaults_key() {
    let exe = "Made Up Game.exe";
    assert_eq!(
        format!("{}\\DllOverrides", exe_defaults_key(exe)),
        override_key(exe)
    );
}
