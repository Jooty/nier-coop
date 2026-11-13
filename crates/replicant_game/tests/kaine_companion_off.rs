use replicant_game::kaine::{
    COMPANION_OFF_FIGHTS, CompanionOff, CompanionOffEvent, CompanionOffStep,
    companion_off_fight_started_by,
};

const FIGHT_PACK: &str = "A_CLIFF_VILLAGE_01";

fn rule_in_open_window() -> CompanionOff {
    let mut rule = CompanionOff::new();
    rule.enter_pack(FIGHT_PACK);
    rule.function_ran("EID_1050_a0020_Start");
    rule
}

#[test]
fn the_companion_is_off_while_the_fight_window_is_open() {
    let mut rule = rule_in_open_window();
    assert_eq!(rule.open_fight(), Some("boss fight 6"));
    assert_eq!(rule.step(true, true), CompanionOffStep::Deactivate);
    for _ in 0..3 {
        assert_eq!(rule.step(true, false), CompanionOffStep::Leave);
    }
    assert!(rule.holds_her());
}

#[test]
fn a_callback_that_turns_her_on_is_undone_within_a_tick() {
    let mut rule = rule_in_open_window();
    assert_eq!(rule.step(true, true), CompanionOffStep::Deactivate);
    assert_eq!(rule.function_ran("EID_1050_a0040_End"), None);
    assert_eq!(rule.step(true, true), CompanionOffStep::Deactivate);
    let report = rule.take_report().expect("window open");
    assert_eq!(report.turned_on_by, vec!["EID_1050_a0040_End".to_owned()]);
    assert!(
        rule.take_report()
            .expect("still open")
            .turned_on_by
            .is_empty()
    );
}

#[test]
fn she_is_given_back_when_the_window_closes() {
    let mut rule = rule_in_open_window();
    rule.step(true, true);
    rule.step(true, false);
    assert!(matches!(
        rule.function_ran("EID_1050_a0070_Start"),
        Some(CompanionOffEvent::Closed { .. })
    ));
    assert_eq!(
        rule.step(true, false),
        CompanionOffStep::GiveBack { active: true }
    );
    assert_eq!(rule.step(true, false), CompanionOffStep::Leave);
    assert!(!rule.holds_her());
}

#[test]
fn she_is_given_back_when_the_players_leave_the_area() {
    let mut rule = rule_in_open_window();
    rule.step(true, true);
    assert!(matches!(
        rule.enter_pack("A_NORTH_FIELD_01"),
        Some(CompanionOffEvent::Closed { .. })
    ));
    assert_eq!(
        rule.step(true, false),
        CompanionOffStep::GiveBack { active: true }
    );
}

#[test]
fn a_window_closed_after_the_game_turned_her_on_again_leaves_her_to_the_game() {
    let mut rule = rule_in_open_window();
    rule.step(true, true);
    rule.enter_pack("A_NORTH_FIELD_01");
    assert_eq!(rule.step(true, true), CompanionOffStep::Leave);
}

#[test]
fn she_stays_off_when_the_scripts_left_her_off() {
    let mut rule = rule_in_open_window();
    assert_eq!(rule.step(true, false), CompanionOffStep::Leave);
    rule.function_ran("EID_1050_a0070_Start");
    assert_eq!(rule.step(true, false), CompanionOffStep::Leave);
}

#[test]
fn no_guest_means_the_rule_does_nothing() {
    let mut rule = rule_in_open_window();
    for active in [true, false, true] {
        assert_eq!(rule.step(false, active), CompanionOffStep::Leave);
    }
    assert!(!rule.holds_her());
}

#[test]
fn a_guest_who_leaves_mid_fight_gets_her_back_for_the_host() {
    let mut rule = rule_in_open_window();
    rule.step(true, true);
    assert_eq!(
        rule.step(false, false),
        CompanionOffStep::GiveBack { active: true }
    );
}

#[test]
fn a_retry_entry_opens_the_window_again() {
    let mut rule = rule_in_open_window();
    rule.function_ran("EID_1050_a0070_Start");
    assert_eq!(rule.open_fight(), None);
    assert!(matches!(
        rule.function_ran("EID_1050_a0045g_Continue"),
        Some(CompanionOffEvent::Opened { .. })
    ));
}

#[test]
fn the_approach_before_the_fight_does_not_open_the_window() {
    let mut rule = CompanionOff::new();
    rule.enter_pack(FIGHT_PACK);
    for name in [
        "EID_1050_a0010_010_Start",
        "EID_1050_a0010_020_Start",
        "EID_1050_a0010_030_Start",
    ] {
        assert_eq!(rule.function_ran(name), None);
    }
    assert_eq!(rule.step(true, true), CompanionOffStep::Leave);
}

#[test]
fn the_fight_table_has_boss_fight_6_with_every_retry_entry() {
    assert_eq!(COMPANION_OFF_FIGHTS.len(), 1);
    for name in [
        "EID_1050_a0020_Continue",
        "EID_1050_a0025g_Continue",
        "EID_1050_a0030_Continue",
        "EID_1050_a0035g_01_Continue",
        "EID_1050_a0035g_02_Continue",
        "EID_1050_a0045g_Continue",
        "EID_1050_a0050_Continue",
        "EID_1050_a0055g_01_Continue",
        "EID_1050_a0055g_02_Continue",
        "EID_1050_a0065g_Continue",
    ] {
        assert_eq!(
            companion_off_fight_started_by(name).map(|fight| fight.name),
            Some("boss fight 6"),
            "{name}"
        );
    }
    assert_eq!(companion_off_fight_started_by("EID_1050_a0070_Start"), None);
}
