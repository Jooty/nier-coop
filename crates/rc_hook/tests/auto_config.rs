use std::fs;
use std::path::PathBuf;

use rc_hook::net::{AUTO_CONFIG_FILE_NAME, AutoConfig, AutoConfigError};

fn scratch_folder(test_name: &str) -> PathBuf {
    let folder = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("test-scratch")
        .join(test_name);
    fs::create_dir_all(&folder).expect("scratch folder is creatable");
    folder
}

#[test]
fn a_link_port_and_name_are_read() {
    let config = AutoConfig::parse("link = 47200\nname = \"Ayla\"\n").expect("valid");
    assert_eq!(config.link, Some(47200));
    assert_eq!(config.name.as_deref(), Some("Ayla"));
}

#[test]
fn the_name_is_optional() {
    let config = AutoConfig::parse("link = 47200\n").expect("valid");
    assert_eq!(config.name, None);
}

#[test]
fn port_zero_is_invalid() {
    assert!(matches!(
        AutoConfig::parse("link = 0\n"),
        Err(AutoConfigError::Invalid(_))
    ));
}

#[test]
fn the_old_in_game_session_keys_are_invalid() {
    assert!(matches!(
        AutoConfig::parse("link = 1\nmode = \"host\"\n"),
        Err(AutoConfigError::Invalid(_))
    ));
    assert!(matches!(
        AutoConfig::parse("link = 1\nticket = \"rcn1abc\"\n"),
        Err(AutoConfigError::Invalid(_))
    ));
}

#[test]
fn missing_file_means_no_launcher_link() {
    let folder = scratch_folder("auto_config_missing");
    let _ = fs::remove_file(folder.join(AUTO_CONFIG_FILE_NAME));
    assert_eq!(AutoConfig::load_from_folder(&folder), Ok(None));
}

#[test]
fn file_in_the_game_folder_is_loaded() {
    let folder = scratch_folder("auto_config_present");
    fs::write(folder.join(AUTO_CONFIG_FILE_NAME), "link = 47200\n").expect("writable");
    let config = AutoConfig::load_from_folder(&folder)
        .expect("valid file")
        .expect("file exists");
    assert_eq!(config.link, Some(47200));
}

#[test]
fn boot_slot_and_block_saves_are_read() {
    let config = AutoConfig::parse("boot_slot = 2\nblock_saves = true\n").expect("valid");
    assert_eq!(config.link, None);
    assert_eq!(config.boot_slot, Some(2));
    assert!(config.block_saves);
}

#[test]
fn boot_slot_and_block_saves_default_to_off() {
    let config = AutoConfig::parse("link = 47200\n").expect("valid");
    assert_eq!(config.boot_slot, None);
    assert!(!config.block_saves);
}

#[test]
fn a_boot_slot_past_the_last_save_slot_is_invalid() {
    assert!(matches!(
        AutoConfig::parse("boot_slot = 32\n"),
        Err(AutoConfigError::Invalid(_))
    ));
}
