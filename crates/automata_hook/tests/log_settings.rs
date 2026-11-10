use automata_hook::log_settings::{DEFAULT_LOG_FILE_NAME, resolve_log_path};

#[test]
fn a_set_variable_wins() {
    assert_eq!(resolve_log_path(Some("a.log".to_owned())), "a.log");
}

#[test]
fn a_missing_or_blank_variable_gives_the_default() {
    assert_eq!(resolve_log_path(None), DEFAULT_LOG_FILE_NAME);
    assert_eq!(
        resolve_log_path(Some("  ".to_owned())),
        DEFAULT_LOG_FILE_NAME
    );
}
