use std::path::Path;

use rc_launcher::install::{
    InstallError, InstallOutcome, Ledger, ModStatus, PROXY_DLL_NAME, cmdline_runs_exe, install_dll,
    mod_mismatch, mod_status, newest_cached_ci_dll, sha256_file, uninstall_dll,
};

use crate::scratch::fresh_dir;

fn setup(name: &str) -> (std::path::PathBuf, std::path::PathBuf, Ledger) {
    let base = fresh_dir(name);
    let game = base.join("game");
    std::fs::create_dir_all(&game).unwrap();
    let ledger = Ledger::new(&base.join("cache/installed.sha256"));
    (base, game, ledger)
}

fn write(path: &Path, bytes: &[u8]) {
    std::fs::write(path, bytes).unwrap();
}

#[test]
fn sha256_matches_the_known_digest_of_abc() {
    let base = fresh_dir("sha_abc");
    write(&base.join("f"), b"abc");
    assert_eq!(
        sha256_file(&base.join("f")).unwrap(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn install_copies_the_dll_once_and_a_repeat_changes_nothing() {
    let (base, game, ledger) = setup("install_twice");
    let source = base.join("rc_hook.dll");
    write(&source, b"build one");
    assert_eq!(
        install_dll(&game, &source, &ledger).unwrap(),
        InstallOutcome::Installed
    );
    assert_eq!(
        install_dll(&game, &source, &ledger).unwrap(),
        InstallOutcome::AlreadyCurrent
    );
    assert_eq!(
        std::fs::read(game.join(PROXY_DLL_NAME)).unwrap(),
        b"build one"
    );
    assert_eq!(
        mod_status(&game, Some(&source), &ledger),
        ModStatus::Current
    );
}

#[test]
fn a_newer_build_replaces_an_older_one_we_installed() {
    let (base, game, ledger) = setup("install_update");
    let source = base.join("rc_hook.dll");
    write(&source, b"build one");
    install_dll(&game, &source, &ledger).unwrap();
    write(&source, b"build two");
    assert_eq!(
        mod_status(&game, Some(&source), &ledger),
        ModStatus::Outdated
    );
    assert_eq!(
        install_dll(&game, &source, &ledger).unwrap(),
        InstallOutcome::Updated
    );
    assert_eq!(
        std::fs::read(game.join(PROXY_DLL_NAME)).unwrap(),
        b"build two"
    );
}

#[test]
fn a_foreign_dll_is_never_overwritten_or_removed() {
    let (base, game, ledger) = setup("install_foreign");
    let source = base.join("rc_hook.dll");
    write(&source, b"ours");
    write(&game.join(PROXY_DLL_NAME), b"someone else's");
    assert!(matches!(
        install_dll(&game, &source, &ledger),
        Err(InstallError::ForeignFileInTheWay)
    ));
    assert!(matches!(
        uninstall_dll(&game, &ledger),
        Err(InstallError::ForeignFileInTheWay)
    ));
    assert_eq!(
        std::fs::read(game.join(PROXY_DLL_NAME)).unwrap(),
        b"someone else's"
    );
    assert_eq!(
        mod_status(&game, Some(&source), &ledger),
        ModStatus::Foreign
    );
}

#[test]
fn uninstall_removes_only_our_dll_and_leaves_other_game_files() {
    let (base, game, ledger) = setup("uninstall");
    let source = base.join("rc_hook.dll");
    write(&source, b"ours");
    write(&game.join("other.dll"), b"keep me");
    install_dll(&game, &source, &ledger).unwrap();
    assert!(uninstall_dll(&game, &ledger).unwrap());
    assert!(!game.join(PROXY_DLL_NAME).exists());
    assert_eq!(std::fs::read(game.join("other.dll")).unwrap(), b"keep me");
    assert!(!uninstall_dll(&game, &ledger).unwrap());
    assert_eq!(
        mod_status(&game, Some(&source), &ledger),
        ModStatus::NotInstalled
    );
}

#[test]
fn newest_cached_ci_build_is_the_highest_run_id_that_has_a_dll() {
    let base = fresh_dir("cached_ci");
    for run in ["99", "1000", "5000"] {
        std::fs::create_dir_all(base.join(run)).unwrap();
    }
    write(&base.join("99/rc_hook.dll"), b"a");
    write(&base.join("1000/rc_hook.dll"), b"b");
    assert_eq!(
        newest_cached_ci_dll(&base),
        Some(base.join("1000/rc_hook.dll"))
    );
}

#[test]
fn game_process_is_recognised_by_its_exe_name_in_the_command_line() {
    let wine = b"Z:\\mnt\\Game\\NieR Replicant ver.1.22474487139.exe\0-arg\0";
    let native = b"/opt/NieR Replicant ver.1.22474487139.exe\0";
    let other = b"/usr/bin/some-other-NieR Replicant ver.1.22474487139.exe\0";
    assert!(cmdline_runs_exe(wine));
    assert!(cmdline_runs_exe(native));
    assert!(!cmdline_runs_exe(other));
    assert!(!cmdline_runs_exe(b"steam\0"));
}

#[test]
fn the_more_recently_written_dll_wins() {
    let base = fresh_dir("newer_dll");
    let old = base.join("old.dll");
    let new = base.join("new.dll");
    std::fs::write(&old, b"1").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(50));
    std::fs::write(&new, b"2").unwrap();
    assert_eq!(
        rc_launcher::install::newer_dll(Some(old.clone()), Some(new.clone())),
        Some(new.clone())
    );
    assert_eq!(
        rc_launcher::install::newer_dll(Some(new.clone()), Some(old.clone())),
        Some(new)
    );
    assert_eq!(
        rc_launcher::install::newer_dll(None, Some(old.clone())),
        Some(old)
    );
}

#[test]
fn a_mod_updated_by_another_launcher_is_not_reported_as_different() {
    let (base, game, ledger) = setup("mismatch_stale");
    let old = base.join("old.dll");
    let new = base.join("new.dll");
    write(&old, b"old build");
    write(&new, b"new build");
    install_dll(&game, &old, &ledger).unwrap();
    assert_eq!(mod_status(&game, Some(&new), &ledger), ModStatus::Outdated);
    assert!(mod_mismatch(&game, Some(&new)).is_some());
    install_dll(&game, &new, &ledger).unwrap();
    assert_eq!(mod_status(&game, Some(&new), &ledger), ModStatus::Current);
    assert_eq!(mod_mismatch(&game, Some(&new)), None);
}

#[test]
fn a_real_difference_names_both_short_hashes() {
    let (base, game, ledger) = setup("mismatch_real");
    let old = base.join("old.dll");
    let new = base.join("new.dll");
    write(&old, b"old build");
    write(&new, b"new build");
    install_dll(&game, &old, &ledger).unwrap();
    let message = mod_mismatch(&game, Some(&new)).unwrap();
    assert!(message.contains(&sha256_file(&old).unwrap()[..8]));
    assert!(message.contains(&sha256_file(&new).unwrap()[..8]));
    assert!(message.contains(PROXY_DLL_NAME));
}
