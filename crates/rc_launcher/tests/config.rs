use std::path::PathBuf;

use rc_launcher::config::LauncherConfig;
use rc_proto::PartnerBody;

use crate::scratch::fresh_dir;

#[test]
fn game_folder_override_survives_a_save_and_load() {
    let dir = fresh_dir("config_roundtrip");
    let path = dir.join("nested/launcher.toml");
    let config = LauncherConfig {
        game_dir: Some(PathBuf::from("/games/nier")),
        player_name: Some("Ayla".to_owned()),
        body: PartnerBody::PrologueCharacter,
        show_distance: true,
        show_ping: false,
        keep_running: false,
        event_log: true,
        show_names: false,
        show_party_frame: true,
        enemy_health_percent: 100,
        enemy_damage_percent: 20,
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

#[test]
fn the_shared_settings_default_to_the_games_own_defaults() {
    let config = LauncherConfig::default();
    assert!(config.keep_running && config.event_log);
    assert!(config.show_names && config.show_party_frame);
    assert!(!config.show_distance && !config.show_ping);
}

#[test]
fn a_config_file_without_the_new_keys_reads_as_all_defaults() {
    assert_eq!(
        LauncherConfig::parse("show_ping = true\n"),
        LauncherConfig {
            show_ping: true,
            ..LauncherConfig::default()
        }
    );
}

#[test]
fn settings_map_to_what_the_game_is_told() {
    let mut config = LauncherConfig::default();
    config.take_player_settings(rc_launcher_core::PlayerSettings {
        keep_running: false,
        event_log: false,
    });
    assert!(!config.player_settings(false).keep_running);
    assert!(config.player_settings(true).keep_running);
    assert!(!config.player_settings(false).event_log);
}

#[test]
fn the_display_settings_are_host_options_not_player_settings() {
    let mut config = LauncherConfig::default();
    config.take_session_options(rc_launcher_core::SessionOptions {
        show_names: true,
        show_party_frame: true,
        show_distance: true,
        show_ping: true,
        ..rc_launcher_core::SessionOptions::default()
    });
    assert!(config.session_options().show_distance && config.session_options().show_ping);
}

#[test]
fn enemy_difficulty_defaults_to_sixty_and_ten_and_leaves_no_key() {
    let config = LauncherConfig::default();
    assert_eq!(config.session_options().enemy_health_percent, 60);
    assert_eq!(config.session_options().enemy_damage_percent, 10);
    let text = config.render();
    assert!(!text.contains("enemy_health_percent"));
    assert!(!text.contains("enemy_damage_percent"));
}

#[test]
fn enemy_difficulty_is_capped_when_the_game_reports_too_much() {
    let mut config = LauncherConfig::default();
    config.take_session_options(rc_launcher_core::SessionOptions {
        enemy_health_percent: 9000,
        enemy_damage_percent: 9000,
        ..rc_launcher_core::SessionOptions::default()
    });
    assert_eq!(config.enemy_health_percent, 300);
    assert_eq!(config.enemy_damage_percent, 50);
}

#[test]
fn the_body_choice_survives_a_save_and_load() {
    let mut config = LauncherConfig::default();
    assert_eq!(config.body, PartnerBody::Nier);
    assert!(!config.render().contains("body"));
    config.body = PartnerBody::PrologueCharacter;
    let loaded = LauncherConfig::parse(&config.render());
    assert_eq!(loaded.body, PartnerBody::PrologueCharacter);
}

#[test]
fn a_config_file_without_a_body_reads_as_nier() {
    assert_eq!(LauncherConfig::parse("").body, PartnerBody::Nier);
}
