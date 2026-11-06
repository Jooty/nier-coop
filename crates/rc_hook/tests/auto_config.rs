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

fn environment(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> + use<> {
    let owned: Vec<(String, String)> = pairs
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect();
    move |key| {
        owned
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.clone())
    }
}

#[test]
fn environment_overrides_the_file() {
    let file = AutoConfig::parse("link = 47200\nname = \"Ayla\"\nboot_slot = 1\n").expect("valid");
    let config = AutoConfig::layered(
        Some(file),
        environment(&[
            ("REPLICANT_COOP_LINK", "47300"),
            ("REPLICANT_COOP_BOOT_SLOT", "3"),
            ("REPLICANT_COOP_BLOCK_SAVES", "1"),
            ("REPLICANT_COOP_NAME", "Guest"),
        ]),
    )
    .expect("valid")
    .expect("present");
    assert_eq!(config.link, Some(47300));
    assert_eq!(config.boot_slot, Some(3));
    assert!(config.block_saves);
    assert_eq!(config.name.as_deref(), Some("Guest"));
}

#[test]
fn file_values_stay_where_the_environment_is_silent() {
    let file = AutoConfig::parse("link = 47200\nname = \"Ayla\"\n").expect("valid");
    let config = AutoConfig::layered(
        Some(file),
        environment(&[("REPLICANT_COOP_BOOT_SLOT", "2")]),
    )
    .expect("valid")
    .expect("present");
    assert_eq!(config.link, Some(47200));
    assert_eq!(config.name.as_deref(), Some("Ayla"));
    assert_eq!(config.boot_slot, Some(2));
}

#[test]
fn environment_can_turn_a_file_block_off() {
    let file = AutoConfig::parse("link = 47200\nblock_saves = true\n").expect("valid");
    let config = AutoConfig::layered(
        Some(file),
        environment(&[("REPLICANT_COOP_BLOCK_SAVES", "false")]),
    )
    .expect("valid")
    .expect("present");
    assert!(!config.block_saves);
}

#[test]
fn environment_alone_makes_a_config() {
    let config = AutoConfig::layered(None, environment(&[("REPLICANT_COOP_LINK", "47300")]))
        .expect("valid")
        .expect("present");
    assert_eq!(config.link, Some(47300));
}

#[test]
fn no_file_and_no_environment_means_no_config() {
    assert_eq!(AutoConfig::layered(None, environment(&[])), Ok(None));
}

#[test]
fn bad_environment_values_are_invalid() {
    for pairs in [
        &[("REPLICANT_COOP_LINK", "abc")][..],
        &[("REPLICANT_COOP_LINK", "0")],
        &[("REPLICANT_COOP_BOOT_SLOT", "99")],
        &[("REPLICANT_COOP_BLOCK_SAVES", "maybe")],
    ] {
        assert!(matches!(
            AutoConfig::layered(None, environment(pairs)),
            Err(AutoConfigError::Invalid(_))
        ));
    }
}

#[test]
fn load_layered_reads_the_file_then_the_environment() {
    let folder = scratch_folder("auto_config_layered");
    fs::write(folder.join(AUTO_CONFIG_FILE_NAME), "link = 47200\n").expect("writable");
    let config =
        AutoConfig::load_layered(&folder, environment(&[("REPLICANT_COOP_LINK", "47301")]))
            .expect("valid")
            .expect("present");
    assert_eq!(config.link, Some(47301));
}
