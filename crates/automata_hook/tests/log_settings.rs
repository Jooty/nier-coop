use std::path::Path;

use automata_hook::log_settings::{
    DEFAULT_LOG_FILE_NAME, absolute_log_path, log_from_config_text, resolve_log_path,
};

#[test]
fn the_environment_wins_over_the_config_file() {
    assert_eq!(
        resolve_log_path(Some("env.log".to_owned()), || Some("file.log".to_owned())),
        "env.log"
    );
}

#[test]
fn the_config_file_is_used_without_an_environment_value() {
    assert_eq!(
        resolve_log_path(None, || Some("file.log".to_owned())),
        "file.log"
    );
    assert_eq!(
        resolve_log_path(Some("  ".to_owned()), || Some("file.log".to_owned())),
        "file.log"
    );
}

#[test]
fn the_default_name_matches_the_other_hook() {
    assert_eq!(resolve_log_path(None, || None), "niertogether.log");
    assert_eq!(DEFAULT_LOG_FILE_NAME, "niertogether.log");
}

#[test]
fn the_log_key_is_read_from_the_config_text() {
    assert_eq!(
        log_from_config_text("link = 4000\nlog = \"game-host.log\"\n"),
        Some("game-host.log".to_owned())
    );
    assert_eq!(log_from_config_text("link = 4000\n"), None);
    assert_eq!(log_from_config_text("not toml ["), None);
}

#[test]
fn a_relative_log_path_is_tied_to_the_exe_folder() {
    let folder = Path::new("games").join("made-up");
    assert_eq!(
        absolute_log_path(DEFAULT_LOG_FILE_NAME.to_owned(), Some(&folder)),
        folder.join(DEFAULT_LOG_FILE_NAME).to_string_lossy()
    );
}

#[test]
fn an_absolute_log_path_or_an_unknown_exe_folder_leaves_the_path_alone() {
    let absolute = std::env::temp_dir().join("coop.log");
    let text = absolute.to_string_lossy().into_owned();
    assert_eq!(
        absolute_log_path(text.clone(), Some(Path::new("games"))),
        text
    );
    assert_eq!(absolute_log_path("coop.log".to_owned(), None), "coop.log");
}
