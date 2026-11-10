use std::path::{Path, PathBuf};

use nc_launcher::proton::{
    environ_uses_prefix, find_proton_files_dir, override_is_set, override_key,
};

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
    assert!(override_key().ends_with("NieR Replicant ver.1.22474487139.exe\\DllOverrides"));
    assert!(override_is_set(
        "    xinput9_1_0    REG_SZ    native,builtin"
    ));
    assert!(!override_is_set("    xinput9_1_0    REG_SZ    builtin"));
}
