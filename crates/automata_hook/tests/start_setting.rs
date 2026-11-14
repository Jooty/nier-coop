use automata_game::autopilot::Autopilot;
use automata_game::character::Character;
use automata_game::enemies::EnemyShare;
use automata_game::partner::PartnerDrive;
use automata_game::save_redirect::SaveRole;
use automata_game::start::StartMode;
use automata_game::story_gate::StoryRule;
use automata_game::story_steps::StoryStepsMode;
use automata_hook::start_setting::STORY_STEPS_ENV;
use automata_hook::start_setting::{SHARED_DEFEAT_ENV, SettingsEnv, choose_game_settings};

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
        "settings: role host, character 2b, partner 9s, start world, start_index 0, start_slot 0, story hold, partner_drive input, link none, shared_enemies on"
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
            .ends_with("partner_drive input, link port 47001, shared_enemies on")
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

#[test]
fn the_autopilot_is_off_unless_the_setting_says_circle() {
    let default = choose_game_settings(SettingsEnv::default(), None);
    assert_eq!(default.autopilot, Autopilot::Off);
    assert!(!default.log_line.contains("autopilot"));
    let from_file = choose_game_settings(SettingsEnv::default(), Some("autopilot = \"circle\"\n"));
    assert_eq!(from_file.autopilot, Autopilot::Circle);
    assert!(from_file.log_line.contains("autopilot circle"));
    let env = SettingsEnv {
        autopilot: Some("off".to_owned()),
        ..SettingsEnv::default()
    };
    let from_env = choose_game_settings(env, Some("autopilot = \"circle\"\n"));
    assert_eq!(from_env.autopilot, Autopilot::Off);
}

#[test]
fn an_unknown_autopilot_word_is_noted_and_stays_off() {
    let env = SettingsEnv {
        autopilot: Some("square".to_owned()),
        ..SettingsEnv::default()
    };
    let settings = choose_game_settings(env, None);
    assert_eq!(settings.autopilot, Autopilot::Off);
    assert!(
        settings
            .log_line
            .contains("unknown autopilot \"square\", using off")
    );
}

#[test]
fn start_together_is_on_unless_the_environment_or_the_file_turns_it_off() {
    let default = choose_game_settings(SettingsEnv::default(), None);
    assert!(default.start_together);
    assert!(!default.log_line.contains("start_together"));
    let from_file =
        choose_game_settings(SettingsEnv::default(), Some("start_together = \"off\"\n"));
    assert!(!from_file.start_together);
    assert!(from_file.log_line.contains(", start_together off"));
    let env = SettingsEnv {
        start_together: Some("on".to_owned()),
        ..SettingsEnv::default()
    };
    let from_env = choose_game_settings(env, Some("start_together = \"off\"\n"));
    assert!(from_env.start_together);
    let env = SettingsEnv {
        start_together: Some("off".to_owned()),
        ..SettingsEnv::default()
    };
    assert!(!choose_game_settings(env, None).start_together);
}

#[test]
fn an_unknown_start_together_value_means_on_and_is_named() {
    let env = SettingsEnv {
        start_together: Some("maybe".to_owned()),
        ..SettingsEnv::default()
    };
    let settings = choose_game_settings(env, None);
    assert!(settings.start_together);
    assert!(
        settings
            .log_line
            .contains("unknown start_together \"maybe\", using on")
    );
}

#[test]
fn one_loading_screen_is_on_unless_the_environment_or_the_file_turns_it_off() {
    let default = choose_game_settings(SettingsEnv::default(), None);
    assert!(default.one_loading_screen);
    assert!(!default.log_line.contains("one_loading_screen"));
    let from_file = choose_game_settings(
        SettingsEnv::default(),
        Some("one_loading_screen = \"off\"\n"),
    );
    assert!(!from_file.one_loading_screen);
    assert!(from_file.start_together);
    assert!(from_file.log_line.contains(", one_loading_screen off"));
    let env = SettingsEnv {
        one_loading_screen: Some("on".to_owned()),
        ..SettingsEnv::default()
    };
    let from_env = choose_game_settings(env, Some("one_loading_screen = \"off\"\n"));
    assert!(from_env.one_loading_screen);
    let env = SettingsEnv {
        one_loading_screen: Some("off".to_owned()),
        ..SettingsEnv::default()
    };
    assert!(!choose_game_settings(env, None).one_loading_screen);
}

#[test]
fn an_unknown_one_loading_screen_value_means_on_and_is_named() {
    let env = SettingsEnv {
        one_loading_screen: Some("twice".to_owned()),
        ..SettingsEnv::default()
    };
    let settings = choose_game_settings(env, None);
    assert!(settings.one_loading_screen);
    assert!(
        settings
            .log_line
            .contains("unknown one_loading_screen \"twice\", using on")
    );
}

#[test]
fn enemies_are_shared_unless_the_environment_or_the_file_says_watch_or_off() {
    let default = choose_game_settings(SettingsEnv::default(), None);
    assert_eq!(default.enemy_share, EnemyShare::On);
    assert!(default.log_line.contains(", shared_enemies on"));
    let from_file =
        choose_game_settings(SettingsEnv::default(), Some("shared_enemies = \"off\"\n"));
    assert_eq!(from_file.enemy_share, EnemyShare::Off);
    assert!(from_file.log_line.contains(", shared_enemies off"));
    let from_a_flag =
        choose_game_settings(SettingsEnv::default(), Some("shared_enemies = false\n"));
    assert_eq!(from_a_flag.enemy_share, EnemyShare::Off);
    let env = SettingsEnv {
        shared_enemies: Some("watch".to_owned()),
        ..SettingsEnv::default()
    };
    let from_env = choose_game_settings(env, Some("shared_enemies = \"off\"\n"));
    assert_eq!(from_env.enemy_share, EnemyShare::Watch);
    assert!(from_env.log_line.contains(", shared_enemies watch"));
}

#[test]
fn an_unknown_shared_enemies_value_means_on_and_is_named() {
    let env = SettingsEnv {
        shared_enemies: Some("some".to_owned()),
        ..SettingsEnv::default()
    };
    let settings = choose_game_settings(env, None);
    assert_eq!(settings.enemy_share, EnemyShare::On);
    assert!(
        settings
            .log_line
            .contains("unknown shared_enemies \"some\", using on")
    );
}

#[test]
fn downed_and_revive_is_on_unless_the_environment_or_the_file_turns_it_off() {
    let default = choose_game_settings(SettingsEnv::default(), None);
    assert!(default.downed_revive);
    assert!(!default.log_line.contains("downed_revive"));
    let from_file = choose_game_settings(SettingsEnv::default(), Some("downed_revive = \"off\"\n"));
    assert!(!from_file.downed_revive);
    assert!(from_file.log_line.contains(", downed_revive off"));
    let from_a_flag = choose_game_settings(SettingsEnv::default(), Some("downed_revive = false\n"));
    assert!(!from_a_flag.downed_revive);
    let env = SettingsEnv {
        downed_revive: Some("on".to_owned()),
        ..SettingsEnv::default()
    };
    assert!(choose_game_settings(env, Some("downed_revive = \"off\"\n")).downed_revive);
    let env = SettingsEnv {
        downed_revive: Some("sometimes".to_owned()),
        ..SettingsEnv::default()
    };
    let unknown = choose_game_settings(env, None);
    assert!(unknown.downed_revive);
    assert!(
        unknown
            .log_line
            .contains("unknown downed_revive \"sometimes\", using on")
    );
}

#[test]
fn every_load_together_is_on_unless_the_environment_or_the_file_turns_it_off() {
    let default = choose_game_settings(SettingsEnv::default(), None);
    assert!(default.every_load_together);
    assert!(!default.log_line.contains("every_load_together"));
    let from_file = choose_game_settings(
        SettingsEnv::default(),
        Some("every_load_together = \"off\"\n"),
    );
    assert!(!from_file.every_load_together);
    assert!(from_file.start_together);
    assert!(from_file.log_line.contains(", every_load_together off"));
    let env = SettingsEnv {
        every_load_together: Some("on".to_owned()),
        ..SettingsEnv::default()
    };
    let from_env = choose_game_settings(env, Some("every_load_together = \"off\"\n"));
    assert!(from_env.every_load_together);
    let env = SettingsEnv {
        every_load_together: Some("off".to_owned()),
        ..SettingsEnv::default()
    };
    assert!(!choose_game_settings(env, None).every_load_together);
}

#[test]
fn an_unknown_every_load_together_value_means_on_and_is_named() {
    let env = SettingsEnv {
        every_load_together: Some("sometimes".to_owned()),
        ..SettingsEnv::default()
    };
    let settings = choose_game_settings(env, None);
    assert!(settings.every_load_together);
    assert!(
        settings
            .log_line
            .contains("unknown every_load_together \"sometimes\", using on")
    );
}

fn starting(mode: &str, story: Option<&str>) -> SettingsEnv {
    SettingsEnv {
        start: Some(mode.to_owned()),
        story: story.map(str::to_owned),
        ..SettingsEnv::default()
    }
}

#[test]
fn a_new_game_or_continue_start_follows_the_host_unless_the_story_is_given() {
    for mode in ["new_game", "continue"] {
        let settings = choose_game_settings(starting(mode, None), None);
        assert_eq!(settings.story, StoryRule::Follow);
        assert!(settings.log_line.contains("story follow"));
    }
}

#[test]
fn story_allow_switches_the_follow_rule_off_for_a_new_game_or_continue_start() {
    for mode in ["new_game", "continue"] {
        let from_env = choose_game_settings(starting(mode, Some("allow")), None);
        assert_eq!(from_env.story, StoryRule::Allow);
        let from_file = choose_game_settings(starting(mode, None), Some("story = \"allow\"\n"));
        assert_eq!(from_file.story, StoryRule::Allow);
    }
}

#[test]
fn story_follow_can_be_given_for_any_start() {
    for mode in ["world", "slot", "new_game", "continue"] {
        let settings = choose_game_settings(starting(mode, Some("follow")), None);
        assert_eq!(settings.story, StoryRule::Follow);
    }
}

#[test]
fn story_hold_with_a_new_game_or_continue_start_is_still_allowed_with_a_note() {
    for mode in ["new_game", "continue"] {
        let settings = choose_game_settings(starting(mode, Some("hold")), None);
        assert_eq!(settings.story, StoryRule::Allow);
        assert!(settings.log_line.contains("story hold does not fit"));
    }
}

#[test]
fn the_other_starts_keep_holding_the_story_by_default() {
    for mode in ["world", "slot"] {
        let settings = choose_game_settings(starting(mode, None), None);
        assert_eq!(settings.story, StoryRule::Hold);
    }
}

#[test]
fn events_together_is_on_unless_the_environment_or_the_file_turns_it_off() {
    let default = choose_game_settings(SettingsEnv::default(), None);
    assert!(default.events_together);
    assert!(default.events_started_for_the_partner);
    assert!(!default.log_line.contains("events_"));
    let from_file =
        choose_game_settings(SettingsEnv::default(), Some("events_together = \"off\"\n"));
    assert!(!from_file.events_together);
    assert!(from_file.events_started_for_the_partner);
    assert!(from_file.log_line.contains(", events_together off"));
    let env = SettingsEnv {
        events_together: Some("on".to_owned()),
        ..SettingsEnv::default()
    };
    assert!(choose_game_settings(env, Some("events_together = \"off\"\n")).events_together);
}

#[test]
fn starts_for_the_partner_can_be_turned_off_alone() {
    let env = SettingsEnv {
        events_started_for_the_partner: Some("off".to_owned()),
        ..SettingsEnv::default()
    };
    let settings = choose_game_settings(env, None);
    assert!(settings.events_together);
    assert!(!settings.events_started_for_the_partner);
    assert!(
        settings
            .log_line
            .contains(", events_started_for_the_partner off")
    );
    let from_file = choose_game_settings(
        SettingsEnv::default(),
        Some("events_started_for_the_partner = false\n"),
    );
    assert!(!from_file.events_started_for_the_partner);
}

#[test]
fn an_unknown_events_together_value_means_on_and_is_named() {
    let env = SettingsEnv {
        events_together: Some("sometimes".to_owned()),
        ..SettingsEnv::default()
    };
    let settings = choose_game_settings(env, None);
    assert!(settings.events_together);
    assert!(
        settings
            .log_line
            .contains("unknown events_together \"sometimes\", using on")
    );
}

#[test]
fn partner_flight_unit_is_on_unless_the_environment_or_the_file_turns_it_off() {
    let default = choose_game_settings(SettingsEnv::default(), None);
    assert!(default.partner_flight_unit);
    assert!(!default.log_line.contains("partner_flight_unit"));
    let from_file = choose_game_settings(
        SettingsEnv::default(),
        Some("partner_flight_unit = \"off\"\n"),
    );
    assert!(!from_file.partner_flight_unit);
    assert!(from_file.every_load_together);
    assert!(from_file.log_line.contains(", partner_flight_unit off"));
    let env = SettingsEnv {
        partner_flight_unit: Some("on".to_owned()),
        ..SettingsEnv::default()
    };
    let from_env = choose_game_settings(env, Some("partner_flight_unit = \"off\"\n"));
    assert!(from_env.partner_flight_unit);
    let env = SettingsEnv {
        partner_flight_unit: Some("off".to_owned()),
        ..SettingsEnv::default()
    };
    assert!(!choose_game_settings(env, None).partner_flight_unit);
}

#[test]
fn an_unknown_partner_flight_unit_value_means_on_and_is_named() {
    let env = SettingsEnv {
        partner_flight_unit: Some("sometimes".to_owned()),
        ..SettingsEnv::default()
    };
    let settings = choose_game_settings(env, None);
    assert!(settings.partner_flight_unit);
    assert!(
        settings
            .log_line
            .contains("unknown partner_flight_unit \"sometimes\", using on")
    );
}

#[test]
fn save_together_is_on_unless_the_environment_or_the_file_turns_it_off() {
    let default = choose_game_settings(SettingsEnv::default(), None);
    assert!(default.save_together);
    assert!(!default.log_line.contains("save_together"));
    let from_file = choose_game_settings(SettingsEnv::default(), Some("save_together = \"off\"\n"));
    assert!(!from_file.save_together);
    assert!(from_file.every_load_together);
    assert!(from_file.log_line.contains(", save_together off"));
    let env = SettingsEnv {
        save_together: Some("on".to_owned()),
        ..SettingsEnv::default()
    };
    let from_env = choose_game_settings(env, Some("save_together = \"off\"\n"));
    assert!(from_env.save_together);
    let env = SettingsEnv {
        save_together: Some("off".to_owned()),
        ..SettingsEnv::default()
    };
    assert!(!choose_game_settings(env, None).save_together);
}

#[test]
fn an_unknown_save_together_value_means_on_and_is_named() {
    let env = SettingsEnv {
        save_together: Some("sometimes".to_owned()),
        ..SettingsEnv::default()
    };
    let settings = choose_game_settings(env, None);
    assert!(settings.save_together);
    assert!(
        settings
            .log_line
            .contains("unknown save_together \"sometimes\", using on")
    );
}

#[test]
fn skip_vote_is_on_unless_the_environment_or_the_file_turns_it_off() {
    let default = choose_game_settings(SettingsEnv::default(), None);
    assert!(default.skip_vote);
    assert!(!default.log_line.contains("skip_vote"));
    let from_file = choose_game_settings(SettingsEnv::default(), Some("skip_vote = \"off\"\n"));
    assert!(!from_file.skip_vote);
    assert!(from_file.events_together);
    assert!(from_file.log_line.contains(", skip_vote off"));
    let env = SettingsEnv {
        skip_vote: Some("off".to_owned()),
        ..SettingsEnv::default()
    };
    assert!(!choose_game_settings(env, None).skip_vote);
    let env = SettingsEnv {
        skip_vote: Some("on".to_owned()),
        ..SettingsEnv::default()
    };
    assert!(choose_game_settings(env, Some("skip_vote = false\n")).skip_vote);
}

#[test]
fn partner_flight_take_over_is_on_unless_the_environment_or_the_file_turns_it_off() {
    let default = choose_game_settings(SettingsEnv::default(), None);
    assert!(default.partner_flight_take_over);
    assert!(!default.log_line.contains("partner_flight_take_over"));
    let from_file = choose_game_settings(
        SettingsEnv::default(),
        Some("partner_flight_take_over = \"off\"\n"),
    );
    assert!(!from_file.partner_flight_take_over);
    assert!(from_file.partner_flight_unit);
    assert!(
        from_file
            .log_line
            .contains(", partner_flight_take_over off")
    );
    let env = SettingsEnv {
        partner_flight_take_over: Some("on".to_owned()),
        ..SettingsEnv::default()
    };
    let from_env = choose_game_settings(env, Some("partner_flight_take_over = \"off\"\n"));
    assert!(from_env.partner_flight_take_over);
    let env = SettingsEnv {
        partner_flight_take_over: Some("off".to_owned()),
        ..SettingsEnv::default()
    };
    assert!(!choose_game_settings(env, None).partner_flight_take_over);
}

#[test]
fn an_unknown_partner_flight_take_over_value_means_on_and_is_named() {
    let env = SettingsEnv {
        partner_flight_take_over: Some("sometimes".to_owned()),
        ..SettingsEnv::default()
    };
    let settings = choose_game_settings(env, None);
    assert!(settings.partner_flight_take_over);
    assert!(
        settings
            .log_line
            .contains("unknown partner_flight_take_over \"sometimes\", using on")
    );
}

#[test]
fn enemy_counts_follow_the_host_is_on_unless_the_environment_or_the_file_turns_it_off() {
    let default = choose_game_settings(SettingsEnv::default(), None);
    assert!(default.enemy_counts_follow_the_host);
    assert!(!default.log_line.contains("enemy_counts_follow_the_host"));
    let from_file = choose_game_settings(
        SettingsEnv::default(),
        Some("enemy_counts_follow_the_host = \"off\"\n"),
    );
    assert!(!from_file.enemy_counts_follow_the_host);
    assert!(from_file.every_load_together);
    assert!(
        from_file
            .log_line
            .contains(", enemy_counts_follow_the_host off")
    );
    let env = SettingsEnv {
        enemy_counts_follow_the_host: Some("on".to_owned()),
        ..SettingsEnv::default()
    };
    let from_env = choose_game_settings(env, Some("enemy_counts_follow_the_host = false\n"));
    assert!(from_env.enemy_counts_follow_the_host);
    let env = SettingsEnv {
        enemy_counts_follow_the_host: Some("off".to_owned()),
        ..SettingsEnv::default()
    };
    assert!(!choose_game_settings(env, None).enemy_counts_follow_the_host);
}

#[test]
fn an_unknown_enemy_counts_follow_the_host_value_means_on_and_is_named() {
    let env = SettingsEnv {
        enemy_counts_follow_the_host: Some("sometimes".to_owned()),
        ..SettingsEnv::default()
    };
    let settings = choose_game_settings(env, None);
    assert!(settings.enemy_counts_follow_the_host);
    assert!(
        settings
            .log_line
            .contains("unknown enemy_counts_follow_the_host \"sometimes\", using on")
    );
}

#[test]
fn shared_defeat_is_on_unless_the_environment_or_the_file_turns_it_off() {
    let default = choose_game_settings(SettingsEnv::default(), None);
    assert!(default.shared_defeat);
    assert!(!default.log_line.contains("shared_defeat"));
    let from_file = choose_game_settings(SettingsEnv::default(), Some("shared_defeat = \"off\"\n"));
    assert!(!from_file.shared_defeat);
    assert!(from_file.downed_revive);
    assert!(from_file.log_line.contains(", shared_defeat off"));
    let env = SettingsEnv {
        shared_defeat: Some("on".to_owned()),
        ..SettingsEnv::default()
    };
    assert!(choose_game_settings(env, Some("shared_defeat = false\n")).shared_defeat);
    let env = SettingsEnv {
        shared_defeat: Some("off".to_owned()),
        ..SettingsEnv::default()
    };
    assert!(!choose_game_settings(env, None).shared_defeat);
}

#[test]
fn an_unknown_shared_defeat_value_means_on_and_is_named() {
    let env = SettingsEnv {
        shared_defeat: Some("sometimes".to_owned()),
        ..SettingsEnv::default()
    };
    let settings = choose_game_settings(env, None);
    assert!(settings.shared_defeat);
    assert!(
        settings
            .log_line
            .contains("unknown shared_defeat \"sometimes\", using on")
    );
}

#[test]
fn the_shared_defeat_setting_reads_its_own_environment_variable() {
    assert_eq!(SHARED_DEFEAT_ENV, "NIER_COOP_SHARED_DEFEAT");
}

#[test]
fn story_steps_watch_unless_the_environment_or_the_file_says_on_or_off() {
    let default = choose_game_settings(SettingsEnv::default(), None);
    assert_eq!(default.story_steps, StoryStepsMode::Watch);
    assert!(!default.log_line.contains("story_steps"));
    let from_file = choose_game_settings(SettingsEnv::default(), Some("story_steps = \"on\"\n"));
    assert_eq!(from_file.story_steps, StoryStepsMode::On);
    assert!(from_file.log_line.contains(", story_steps on"));
    let from_a_flag = choose_game_settings(SettingsEnv::default(), Some("story_steps = false\n"));
    assert_eq!(from_a_flag.story_steps, StoryStepsMode::Off);
    assert!(from_a_flag.log_line.contains(", story_steps off"));
    let env = SettingsEnv {
        story_steps: Some("watch".to_owned()),
        ..SettingsEnv::default()
    };
    let from_env = choose_game_settings(env, Some("story_steps = \"on\"\n"));
    assert_eq!(from_env.story_steps, StoryStepsMode::Watch);
}

#[test]
fn an_unknown_story_steps_value_means_watch_and_is_named() {
    let env = SettingsEnv {
        story_steps: Some("sometimes".to_owned()),
        ..SettingsEnv::default()
    };
    let settings = choose_game_settings(env, None);
    assert_eq!(settings.story_steps, StoryStepsMode::Watch);
    assert!(
        settings
            .log_line
            .contains("unknown story_steps \"sometimes\", using watch")
    );
}

#[test]
fn the_story_steps_setting_reads_its_own_environment_variable() {
    assert_eq!(STORY_STEPS_ENV, "NIER_COOP_STORY_STEPS");
}

#[test]
fn a_new_game_or_continue_start_with_story_allow_says_the_host_does_not_lead() {
    for mode in ["new_game", "continue"] {
        let settings = choose_game_settings(starting(mode, Some("allow")), None);
        assert!(
            settings
                .log_line
                .contains("the host does not lead the story in this session")
        );
        let followed = choose_game_settings(starting(mode, Some("follow")), None);
        assert!(!followed.log_line.contains("does not lead"));
    }
    let world = choose_game_settings(starting("world", Some("allow")), None);
    assert!(!world.log_line.contains("does not lead"));
}
