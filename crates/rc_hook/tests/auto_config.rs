use std::fs;
use std::path::PathBuf;

use rc_hook::net::{AUTO_CONFIG_FILE_NAME, AutoConfig, AutoConfigError, AutoMode};

fn scratch_folder(test_name: &str) -> PathBuf {
    let folder = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("test-scratch")
        .join(test_name);
    fs::create_dir_all(&folder).expect("scratch folder is creatable");
    folder
}

#[test]
fn host_mode_needs_no_ticket() {
    let config = AutoConfig::parse("mode = \"host\"\nname = \"Ayla\"\n").expect("valid");
    assert_eq!(config.mode, Some(AutoMode::Host));
    assert_eq!(config.name.as_deref(), Some("Ayla"));
    assert_eq!(config.ticket, None);
}

#[test]
fn join_mode_reads_the_ticket() {
    let config = AutoConfig::parse("mode = \"join\"\nticket = \"rcn1abc\"\n").expect("valid");
    assert_eq!(config.mode, Some(AutoMode::Join));
    assert_eq!(config.ticket.as_deref(), Some("rcn1abc"));
}

#[test]
fn join_mode_without_a_ticket_is_refused() {
    assert_eq!(
        AutoConfig::parse("mode = \"join\"\n"),
        Err(AutoConfigError::JoinWithoutTicket)
    );
    assert_eq!(
        AutoConfig::parse("mode = \"join\"\nticket = \"  \"\n"),
        Err(AutoConfigError::JoinWithoutTicket)
    );
}

#[test]
fn unknown_mode_and_unknown_keys_are_refused() {
    assert!(matches!(
        AutoConfig::parse("mode = \"spectate\"\n"),
        Err(AutoConfigError::Invalid(_))
    ));
    assert!(matches!(
        AutoConfig::parse("mode = \"host\"\nport = 1\n"),
        Err(AutoConfigError::Invalid(_))
    ));
}

#[test]
fn missing_file_means_no_auto_mode() {
    let folder = scratch_folder("auto_config_missing");
    let _ = fs::remove_file(folder.join(AUTO_CONFIG_FILE_NAME));
    assert_eq!(AutoConfig::load_from_folder(&folder), Ok(None));
}

#[test]
fn file_in_the_game_folder_is_loaded() {
    let folder = scratch_folder("auto_config_present");
    fs::write(folder.join(AUTO_CONFIG_FILE_NAME), "mode = \"host\"\n").expect("writable");
    let config = AutoConfig::load_from_folder(&folder)
        .expect("valid file")
        .expect("file exists");
    assert_eq!(config.mode, Some(AutoMode::Host));
}

#[test]
fn a_harness_only_file_is_valid_and_turns_the_flag_on() {
    let config = AutoConfig::parse("harness = true\n").expect("valid");
    assert_eq!(config.mode, None);
    assert!(config.harness);
}
