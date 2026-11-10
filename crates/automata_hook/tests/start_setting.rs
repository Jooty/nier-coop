use automata_game::character::Character;
use automata_game::partner::PartnerDrive;
use automata_game::save_redirect::SaveRole;
use automata_game::start::StartMode;
use automata_game::story_gate::StoryRule;
use automata_hook::start_setting::{SettingsEnv, choose_game_settings};

#[test]
fn nothing_given_means_the_defaults_for_the_host() {
    let settings = choose_game_settings(SettingsEnv::default(), None);
    assert_eq!(settings.role, SaveRole::Host);
    assert_eq!(settings.character, Character::TwoB);
    assert_eq!(settings.partner, Character::NineS);
    assert_eq!(settings.mode, StartMode::World);
    assert_eq!(settings.index, 0);
    assert_eq!(settings.slot, 0);
}

#[test]
fn the_guest_defaults_to_nine_s_with_two_b_as_partner() {
    let env = SettingsEnv {
        role: Some("guest".to_owned()),
        ..SettingsEnv::default()
    };
    let settings = choose_game_settings(env, None);
    assert_eq!(settings.character, Character::NineS);
    assert_eq!(settings.partner, Character::TwoB);
}

#[test]
fn the_link_file_is_read_and_the_environment_wins() {
    let file = "role = \"guest\"\ncharacter = \"a2\"\npartner_character = \"2b\"\nstart = \"slot\"\nstart_index = 3\nstart_slot = 1\n";
    let from_file = choose_game_settings(SettingsEnv::default(), Some(file));
    assert_eq!(from_file.character, Character::A2);
    assert_eq!(from_file.partner, Character::TwoB);
    assert_eq!(from_file.mode, StartMode::Slot);
    assert_eq!(from_file.index, 3);
    assert_eq!(from_file.slot, 1);
    let env = SettingsEnv {
        start: Some("title".to_owned()),
        start_index: Some("5".to_owned()),
        ..SettingsEnv::default()
    };
    let from_env = choose_game_settings(env, Some(file));
    assert_eq!(from_env.mode, StartMode::Title);
    assert_eq!(from_env.index, 5);
    assert_eq!(from_env.slot, 1);
}

#[test]
fn a_partner_equal_to_the_character_becomes_the_first_other_one() {
    let env = SettingsEnv {
        character: Some("2b".to_owned()),
        partner_character: Some("2b".to_owned()),
        ..SettingsEnv::default()
    };
    let settings = choose_game_settings(env, None);
    assert_eq!(settings.partner, Character::NineS);
    assert_ne!(settings.partner, settings.character);
}

#[test]
fn the_default_partner_is_moved_off_the_characters_own_body() {
    let env = SettingsEnv {
        character: Some("9s".to_owned()),
        ..SettingsEnv::default()
    };
    let settings = choose_game_settings(env, None);
    assert_eq!(settings.character, Character::NineS);
    assert_eq!(settings.partner, Character::TwoB);
}

#[test]
fn unknown_values_fall_back_and_are_named_in_the_log_line() {
    let env = SettingsEnv {
        partner_character: Some("pod".to_owned()),
        start: Some("sky".to_owned()),
        start_index: Some("-2".to_owned()),
        ..SettingsEnv::default()
    };
    let settings = choose_game_settings(env, None);
    assert_eq!(settings.partner, Character::NineS);
    assert_eq!(settings.mode, StartMode::World);
    assert_eq!(settings.index, 0);
    assert!(
        settings
            .log_line
            .contains("unknown partner_character \"pod\"")
    );
    assert!(settings.log_line.contains("unknown start \"sky\""));
    assert!(settings.log_line.contains("unreadable start_index \"-2\""));
}

#[test]
fn one_log_line_holds_every_setting() {
    let settings = choose_game_settings(SettingsEnv::default(), None);
    assert_eq!(
        settings.log_line,
        "settings: role host, character 2b, partner 9s, start world, start_index 0, start_slot 0, story hold, partner_drive input, link none"
    );
}

#[test]
fn the_link_port_and_the_partner_drive_come_from_the_file_or_the_environment() {
    let file = "link = 47001\npartner_drive = \"input\"\n";
    let from_file = choose_game_settings(SettingsEnv::default(), Some(file));
    assert_eq!(from_file.link_port, Some(47001));
    assert_eq!(from_file.partner_drive, PartnerDrive::Input);
    assert!(
        from_file
            .log_line
            .ends_with("partner_drive input, link port 47001")
    );
    let env = SettingsEnv {
        link: Some("47002".to_owned()),
        partner_drive: Some("state".to_owned()),
        ..SettingsEnv::default()
    };
    let from_env = choose_game_settings(env, Some(file));
    assert_eq!(from_env.link_port, Some(47002));
    assert_eq!(from_env.partner_drive, PartnerDrive::State);
}

#[test]
fn no_link_port_means_co_op_is_off_and_a_bad_one_is_named() {
    let none = choose_game_settings(SettingsEnv::default(), None);
    assert_eq!(none.link_port, None);
    let env = SettingsEnv {
        link: Some("0".to_owned()),
        partner_drive: Some("pad".to_owned()),
        ..SettingsEnv::default()
    };
    let bad = choose_game_settings(env, None);
    assert_eq!(bad.link_port, None);
    assert!(bad.log_line.contains("unreadable link \"0\""));
    assert!(bad.log_line.contains("unknown partner_drive \"pad\""));
}

#[test]
fn the_start_config_carries_the_character_as_hero_and_the_partner() {
    let settings = choose_game_settings(SettingsEnv::default(), None);
    let config = settings.start_config();
    assert_eq!(config.hero, Character::TwoB);
    assert_eq!(config.partner, Character::NineS);
}

#[test]
fn the_story_is_held_unless_the_setting_says_allow() {
    let default = choose_game_settings(SettingsEnv::default(), None);
    assert_eq!(default.story, StoryRule::Hold);
    assert!(default.log_line.contains("story hold"));
    let from_file = choose_game_settings(SettingsEnv::default(), Some("story = \"allow\"\n"));
    assert_eq!(from_file.story, StoryRule::Allow);
    assert!(from_file.log_line.contains("story allow"));
    let env = SettingsEnv {
        story: Some("hold".to_owned()),
        ..SettingsEnv::default()
    };
    let from_env = choose_game_settings(env, Some("story = \"allow\"\n"));
    assert_eq!(from_env.story, StoryRule::Hold);
}

#[test]
fn an_unknown_story_word_is_noted_and_holds() {
    let env = SettingsEnv {
        story: Some("free".to_owned()),
        ..SettingsEnv::default()
    };
    let settings = choose_game_settings(env, None);
    assert_eq!(settings.story, StoryRule::Hold);
    assert!(
        settings
            .log_line
            .contains("unknown story \"free\", using hold")
    );
}

#[test]
fn a_slot_above_three_is_noted_for_slot_start_only() {
    let slot_env = SettingsEnv {
        start: Some("slot".to_owned()),
        start_slot: Some("4".to_owned()),
        ..SettingsEnv::default()
    };
    let settings = choose_game_settings(slot_env, None);
    assert_eq!(settings.slot, 4);
    assert!(
        settings
            .log_line
            .contains("start_slot 4 is not a save slot (0 to 3)")
    );
    let world_env = SettingsEnv {
        start_slot: Some("4".to_owned()),
        ..SettingsEnv::default()
    };
    assert!(
        !choose_game_settings(world_env, None)
            .log_line
            .contains("not a save slot")
    );
}
