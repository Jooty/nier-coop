use replicant_game::kaine::{
    COMPANION_LENT_AFTER_A_CALL_MS, COMPANION_LENT_AFTER_HER_SCRIPT_MODE_MS,
    COMPANION_OFF_EVERYWHERE_ROW_NAME, CompanionCallLend, CompanionLendReason, CompanionOff,
    CompanionOffStep, CompanionOffTick, DRIVER_ROW_PACK_FOR_A_LIBRARY_FUNCTION,
    FUNCTIONS_THAT_DRIVE_HER_FROM_SCRIPTS, function_drives_her,
};

const FIELD_PACK: &str = "A_MOUNT_ROBOT_01";
const MANSION_PACK: &str = "A_SOUTH_MANSION_01";
const FIGHT_PACK: &str = "A_CLIFF_VILLAGE_01";
const ELEVATOR_SEEK: &str = "MoveElvEnterPosPlayerAndFriends";

fn tick(now_ms: u64, guest_in_session: bool, active_now: bool) -> CompanionOffTick {
    CompanionOffTick {
        guest_in_session,
        active_now,
        event_scene: false,
        her_script_mode: false,
        now_ms,
    }
}

fn script_mode_tick(now_ms: u64, active_now: bool, script_mode: bool) -> CompanionOffTick {
    CompanionOffTick {
        her_script_mode: script_mode,
        ..tick(now_ms, true, active_now)
    }
}

fn general_rule_in(pack: &str) -> CompanionOff {
    let mut rule = CompanionOff::new();
    rule.keep_her_off_everywhere(true);
    rule.enter_pack(pack);
    rule
}

fn general_rule_with_her_held_off() -> CompanionOff {
    let mut rule = general_rule_in(FIELD_PACK);
    assert_eq!(rule.step(tick(0, true, true)), CompanionOffStep::Deactivate);
    assert_eq!(rule.step(tick(16, true, false)), CompanionOffStep::Leave);
    rule
}

#[test]
fn outside_every_listed_fight_she_is_off_while_a_guest_is_in_the_session() {
    let mut rule = general_rule_with_her_held_off();
    assert_eq!(rule.open_fight(), None);
    assert!(rule.keeps_her_off());
    assert!(rule.holds_her());
    assert_eq!(
        rule.step(tick(32, true, true)),
        CompanionOffStep::Deactivate
    );
    let report = rule
        .take_report()
        .expect("the general rule reports once a second");
    assert_eq!(report.fight, COMPANION_OFF_EVERYWHERE_ROW_NAME);
    assert_eq!(report.lent, None);
}

#[test]
fn with_the_setting_off_she_is_off_only_in_listed_fights() {
    let mut rule = CompanionOff::new();
    rule.enter_pack(FIELD_PACK);
    assert!(!rule.keeps_her_off_everywhere());
    assert_eq!(rule.step(tick(0, true, true)), CompanionOffStep::Leave);
    assert!(!rule.keeps_her_off());
    assert_eq!(rule.take_report(), None);
}

#[test]
fn no_guest_means_the_general_rule_does_nothing() {
    let mut rule = general_rule_in(FIELD_PACK);
    assert_eq!(rule.step(tick(0, false, true)), CompanionOffStep::Leave);
    assert!(!rule.keeps_her_off());
    assert_eq!(rule.take_report(), None);
}

#[test]
fn she_is_given_back_as_the_scripts_left_her_when_the_guest_leaves() {
    let mut rule = general_rule_with_her_held_off();
    assert_eq!(
        rule.step(tick(32, false, false)),
        CompanionOffStep::GiveBack { active: true }
    );
    assert!(!rule.keeps_her_off());
}

#[test]
fn a_listed_fight_that_closes_leaves_her_off_under_the_general_rule() {
    let mut rule = general_rule_in(FIGHT_PACK);
    rule.function_ran("EID_1050_a0020_Start");
    assert_eq!(rule.open_fight(), Some("boss fight 6"));
    assert_eq!(rule.step(tick(0, true, true)), CompanionOffStep::Deactivate);
    rule.function_ran("EID_1050_a0070_Start");
    assert_eq!(rule.open_fight(), None);
    assert_eq!(rule.step(tick(16, true, false)), CompanionOffStep::Leave);
    assert_eq!(rule.acting_row(), Some(COMPANION_OFF_EVERYWHERE_ROW_NAME));
    assert_eq!(
        rule.step(tick(32, true, true)),
        CompanionOffStep::Deactivate
    );
}

#[test]
fn her_script_mode_lends_her_until_a_moment_after_it_ends() {
    let mut rule = general_rule_with_her_held_off();
    rule.function_ran("ScriptMode_to_Kaine_01");
    assert_eq!(
        rule.step(script_mode_tick(1_000, false, true)),
        CompanionOffStep::Lend {
            write_active: true,
            why: CompanionLendReason::ScriptMode("ScriptMode_to_Kaine_01".to_owned())
        }
    );
    assert_eq!(
        rule.step(script_mode_tick(60_000, true, true)),
        CompanionOffStep::Leave
    );
    assert_eq!(
        rule.step(script_mode_tick(61_000, true, false)),
        CompanionOffStep::Leave
    );
    assert_eq!(
        rule.step(script_mode_tick(
            61_000 + COMPANION_LENT_AFTER_HER_SCRIPT_MODE_MS,
            true,
            false
        )),
        CompanionOffStep::LendOver {
            deactivate: true,
            why: CompanionLendReason::ScriptMode("ScriptMode_to_Kaine_01".to_owned())
        }
    );
}

#[test]
fn her_script_mode_leaves_her_off_when_the_scripts_left_her_off() {
    let mut rule = general_rule_in(FIELD_PACK);
    assert_eq!(rule.step(tick(0, true, false)), CompanionOffStep::Leave);
    assert_eq!(
        rule.step(script_mode_tick(16, false, true)),
        CompanionOffStep::Lend {
            write_active: false,
            why: CompanionLendReason::ScriptMode("no script function seen".to_owned())
        }
    );
}

#[test]
fn a_function_that_drives_her_is_lent_her_before_it_runs() {
    let mut rule = general_rule_with_her_held_off();
    rule.function_ran(ELEVATOR_SEEK);
    assert_eq!(
        rule.function_addresses_her(ELEVATOR_SEEK, tick(5_000, true, false)),
        Some(CompanionCallLend {
            fight: COMPANION_OFF_EVERYWHERE_ROW_NAME,
            write_active: true
        })
    );
    assert_eq!(
        rule.lent_for(),
        Some(&CompanionLendReason::Call(ELEVATOR_SEEK.to_owned()))
    );
    assert_eq!(
        rule.step(tick(5_000 + COMPANION_LENT_AFTER_A_CALL_MS, true, true)),
        CompanionOffStep::LendOver {
            deactivate: true,
            why: CompanionLendReason::Call(ELEVATOR_SEEK.to_owned())
        }
    );
}

#[test]
fn a_function_that_drives_her_only_in_another_pack_lends_nothing_here() {
    let mut rule = general_rule_with_her_held_off();
    rule.function_ran("ScriptMode_to_Kaine_01");
    assert_eq!(
        rule.function_addresses_her("ScriptMode_to_Kaine_01", tick(5_000, true, false)),
        None
    );
    assert_eq!(rule.lent_for(), None);
}

#[test]
fn a_call_inside_her_script_mode_does_not_cut_the_lend_short() {
    let mut rule = general_rule_with_her_held_off();
    rule.step(script_mode_tick(1_000, false, true));
    rule.function_ran(ELEVATOR_SEEK);
    rule.function_addresses_her(ELEVATOR_SEEK, script_mode_tick(1_016, true, true));
    assert_eq!(
        rule.step(script_mode_tick(
            1_016 + COMPANION_LENT_AFTER_A_CALL_MS,
            true,
            true
        )),
        CompanionOffStep::Leave
    );
    assert!(matches!(
        rule.lent_for(),
        Some(CompanionLendReason::ScriptMode(_))
    ));
}

#[test]
fn a_call_lend_turns_into_a_script_mode_lend_that_lasts_while_it_is_on() {
    let mut rule = general_rule_with_her_held_off();
    rule.function_ran(ELEVATOR_SEEK);
    rule.function_addresses_her(ELEVATOR_SEEK, tick(1_000, true, false));
    rule.step(script_mode_tick(1_016, true, true));
    assert_eq!(
        rule.step(script_mode_tick(30_000, true, true)),
        CompanionOffStep::Leave
    );
    assert_eq!(
        rule.lent_for(),
        Some(&CompanionLendReason::ScriptMode(ELEVATOR_SEEK.to_owned()))
    );
}

#[test]
fn in_a_listed_fight_her_script_mode_lends_nothing() {
    let mut rule = general_rule_in(FIGHT_PACK);
    rule.function_ran("EID_1050_a0020_Start");
    assert_eq!(rule.step(tick(0, true, true)), CompanionOffStep::Deactivate);
    assert_eq!(
        rule.step(script_mode_tick(16, true, true)),
        CompanionOffStep::Deactivate
    );
    assert_eq!(rule.lent_for(), None);
}

#[test]
fn the_driver_table_is_sorted_once_so_every_name_is_found() {
    assert!(
        FUNCTIONS_THAT_DRIVE_HER_FROM_SCRIPTS
            .windows(2)
            .all(|pair| pair[0] < pair[1])
    );
    assert!(
        FUNCTIONS_THAT_DRIVE_HER_FROM_SCRIPTS
            .iter()
            .all(|(pack, name)| function_drives_her(pack, name))
    );
    assert!(function_drives_her(FIELD_PACK, ELEVATOR_SEEK));
    assert!(function_drives_her(MANSION_PACK, "ScriptMode_to_Kaine_01"));
    assert!(!function_drives_her(FIELD_PACK, "ScriptMode_to_Kaine_01"));
    assert!(!function_drives_her(FIELD_PACK, "LIB_KAINE_SETTING"));
}

#[test]
fn a_library_function_that_drives_her_counts_in_every_pack() {
    assert!(
        FUNCTIONS_THAT_DRIVE_HER_FROM_SCRIPTS
            .iter()
            .any(|(pack, _)| *pack == DRIVER_ROW_PACK_FOR_A_LIBRARY_FUNCTION)
    );
    assert!(function_drives_her(FIELD_PACK, "LIB_KAINE_Lip_open"));
    assert!(function_drives_her(MANSION_PACK, "LIB_KAINE_Lip_open"));
}
