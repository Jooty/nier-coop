use automata_hook::log_settings::{DEFAULT_LOG_FILE_NAME, log_from_config_text, resolve_log_path};

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
    assert_eq!(resolve_log_path(None, || None), "nier-coop.log");
    assert_eq!(DEFAULT_LOG_FILE_NAME, "nier-coop.log");
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
