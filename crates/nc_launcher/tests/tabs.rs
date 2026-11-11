use nc_launcher::app::{RESTART_TAB_LOGS, Tab};

#[test]
fn logs_is_a_tab() {
    assert!(Tab::ALL.iter().any(|(tab, _)| *tab == Tab::Logs));
}

#[test]
fn the_logs_restart_key_opens_the_logs_tab() {
    assert_eq!(Tab::from_restart_value(Some(RESTART_TAB_LOGS)), Tab::Logs);
}
