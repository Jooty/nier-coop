use std::path::PathBuf;

use rc_launcher::config::LauncherConfig;

use crate::scratch::fresh_dir;

#[test]
fn game_folder_override_survives_a_save_and_load() {
    let dir = fresh_dir("config_roundtrip");
    let path = dir.join("nested/launcher.toml");
    let config = LauncherConfig {
        game_dir: Some(PathBuf::from("/games/nier")),
        player_name: Some("Ayla".to_owned()),
    };
    config.save(&path).unwrap();
    assert_eq!(LauncherConfig::load(&path), config);
}

#[test]
fn missing_or_broken_config_gives_the_default() {
    let dir = fresh_dir("config_broken");
    assert_eq!(
        LauncherConfig::load(&dir.join("none.toml")),
        LauncherConfig::default()
    );
    assert_eq!(
        LauncherConfig::parse("game_dir = [1"),
        LauncherConfig::default()
    );
}

#[test]
fn empty_override_writes_no_key() {
    assert_eq!(LauncherConfig::default().render().trim(), "");
}
