use std::collections::BTreeMap;
use std::path::PathBuf;

use nc_launcher::config::LauncherConfig;
use nc_launcher::game_link::{StartChoice, StartSetting};
use nc_launcher_core::REPLICANT;
use replicant_proto::PartnerBody;

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
        check_for_updates: false,
        update_url: Some("http://localhost/test/manifest.json".to_owned()),
        send_logs: false,
        install_id: Some("0123456789abcdef0123456789abcdef".to_owned()),
        start: StartSetting {
            choice: StartChoice::Slot,
            index: 3,
            slot: 2,
            story_allowed: true,
        },
        game_dirs: BTreeMap::from([("made-up".to_owned(), PathBuf::from("/games/other"))]),
        character_ids: BTreeMap::from([("made-up".to_owned(), "red".to_owned())]),
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
    config.take_player_settings(nc_launcher_core::PlayerSettings {
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
    config.take_session_options(nc_launcher_core::SessionOptions {
        show_names: true,
        show_party_frame: true,
        show_distance: true,
        show_ping: true,
        ..nc_launcher_core::SessionOptions::default()
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
    config.take_session_options(nc_launcher_core::SessionOptions {
        enemy_health_percent: 9000,
        enemy_damage_percent: 9000,
        ..nc_launcher_core::SessionOptions::default()
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

#[test]
fn the_old_folder_is_renamed_only_when_the_new_one_is_missing() {
    use nc_launcher::config::{FolderMove, plan_folder_move};
    assert_eq!(plan_folder_move(true, false), FolderMove::RenameOldToNew);
    assert_eq!(plan_folder_move(true, true), FolderMove::Nothing);
    assert_eq!(plan_folder_move(false, false), FolderMove::Nothing);
    assert_eq!(plan_folder_move(false, true), FolderMove::Nothing);
}

#[cfg(unix)]
#[test]
fn moving_the_old_folder_keeps_its_files_and_leaves_a_link_behind() {
    use nc_launcher::config::{FolderMove, move_folder};
    let dir = fresh_dir("folder_move");
    let old = dir.join("replicant-coop");
    let new = dir.join("nier-coop");
    std::fs::create_dir_all(&old).unwrap();
    std::fs::write(old.join("launcher.toml"), "keep").unwrap();
    assert_eq!(move_folder(&old, &new).unwrap(), FolderMove::RenameOldToNew);
    assert_eq!(
        std::fs::read_to_string(new.join("launcher.toml")).unwrap(),
        "keep"
    );
    assert_eq!(
        std::fs::read_to_string(old.join("launcher.toml")).unwrap(),
        "keep"
    );
    assert_eq!(move_folder(&old, &new).unwrap(), FolderMove::Nothing);
}

#[cfg(unix)]
#[test]
fn an_existing_new_folder_is_never_overwritten() {
    use nc_launcher::config::{FolderMove, move_folder};
    let dir = fresh_dir("folder_move_keeps_new");
    let old = dir.join("replicant-coop");
    let new = dir.join("nier-coop");
    std::fs::create_dir_all(&old).unwrap();
    std::fs::create_dir_all(&new).unwrap();
    std::fs::write(old.join("launcher.toml"), "old").unwrap();
    std::fs::write(new.join("launcher.toml"), "new").unwrap();
    assert_eq!(move_folder(&old, &new).unwrap(), FolderMove::Nothing);
    assert_eq!(
        std::fs::read_to_string(new.join("launcher.toml")).unwrap(),
        "new"
    );
    assert_eq!(
        std::fs::read_to_string(old.join("launcher.toml")).unwrap(),
        "old"
    );
}

#[test]
fn each_game_keeps_its_own_folder_override() {
    let other = nc_launcher_core::GameProfile {
        game_id: "made-up",
        ..REPLICANT
    };
    let mut config = LauncherConfig::default();
    config.set_game_dir_for(&REPLICANT, Some(PathBuf::from("/games/first")));
    config.set_game_dir_for(&other, Some(PathBuf::from("/games/second")));
    assert_eq!(
        config.game_dir_for(&REPLICANT),
        Some(&PathBuf::from("/games/first"))
    );
    assert_eq!(
        config.game_dir_for(&other),
        Some(&PathBuf::from("/games/second"))
    );
    config.set_game_dir_for(&other, None);
    assert_eq!(config.game_dir_for(&other), None);
    assert_eq!(
        config.game_dir_for(&REPLICANT),
        Some(&PathBuf::from("/games/first"))
    );
}
