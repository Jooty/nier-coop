use replicant_game::stay_together::PARTNER_TIMEOUT_MS;
use replicant_game::talk_guard::{
    IMPORTANT_TALK_BASES, IMPORTANT_TALK_IDS, PressAsk, PressVerdict, TalkRegistry,
    important_base_of_talk_id, is_important_talk, judge_press,
};
use replicant_game::zone_rules::{Companion, Placement, ZoneRadius};

const IMPORTANT_START: &str = "EID_0100_a0038g_Start";
const ORDINARY_START: &str = "EID_0100_zzzz_Start";
const PHASE: u16 = 3;
const LOCAL: Placement = Placement {
    phase: PHASE,
    position: [0.0, 0.0, 0.0],
};
const ACTOR: i32 = 7;

fn partner_at(x: f32, phase: u16, age_ms: u64) -> Companion {
    Companion {
        name: "Sam".to_owned(),
        placement: Some(Placement {
            phase,
            position: [x, 0.0, 0.0],
        }),
        age_ms,
    }
}

fn verdict_with(connected: bool, game_open: bool, partner: Option<&Companion>) -> PressVerdict {
    judge_press(&PressAsk {
        connected,
        partner_game_open: game_open,
        local: Some(LOCAL),
        partner,
        radius: ZoneRadius::default(),
    })
}

#[test]
fn the_generated_table_holds_the_next_day_cutscene() {
    assert!(IMPORTANT_TALK_BASES.contains(&"EID_0100_a0038g"));
    assert!(is_important_talk(IMPORTANT_START));
}

#[test]
fn the_generated_table_is_sorted_and_free_of_duplicates() {
    assert!(
        IMPORTANT_TALK_BASES
            .windows(2)
            .all(|pair| pair[0] < pair[1])
    );
}

#[test]
fn a_talk_that_is_not_listed_is_not_important() {
    assert!(!is_important_talk(ORDINARY_START));
}

#[test]
fn an_important_press_with_a_near_partner_is_allowed() {
    let near = partner_at(5.0, PHASE, 0);
    assert_eq!(verdict_with(true, true, Some(&near)), PressVerdict::Allow);
}

#[test]
fn an_important_press_with_a_far_partner_waits() {
    let far = partner_at(100.0, PHASE, 0);
    let PressVerdict::Wait {
        partner,
        distance_units,
    } = verdict_with(true, true, Some(&far))
    else {
        panic!("expected a wait");
    };
    assert_eq!(partner, "Sam");
    assert!((distance_units - 100.0).abs() < 0.01);
}

#[test]
fn the_wait_text_names_the_partner() {
    let far = partner_at(100.0, PHASE, 0);
    let text = verdict_with(true, true, Some(&far)).prompt_text();
    assert!(text.is_some_and(|text| text.contains("Sam")));
}

#[test]
fn a_partner_in_another_phase_never_makes_the_press_wait() {
    let elsewhere = partner_at(100.0, PHASE + 1, 0);
    assert_eq!(
        verdict_with(true, true, Some(&elsewhere)),
        PressVerdict::Allow
    );
}

#[test]
fn an_offline_closed_or_silent_partner_never_makes_the_press_wait() {
    let far = partner_at(100.0, PHASE, 0);
    assert_eq!(verdict_with(false, true, Some(&far)), PressVerdict::Allow);
    assert_eq!(verdict_with(true, false, Some(&far)), PressVerdict::Allow);
    let silent = partner_at(100.0, PHASE, PARTNER_TIMEOUT_MS + 1);
    assert_eq!(verdict_with(true, true, Some(&silent)), PressVerdict::Allow);
    assert_eq!(verdict_with(true, true, None), PressVerdict::Allow);
}

#[test]
fn a_registered_talk_is_found_by_its_actor() {
    let mut registry = TalkRegistry::default();
    registry.enter_phase("A_CENTER_VILLAGE_01");
    registry.register_name(ACTOR, 1, IMPORTANT_START);
    assert_eq!(registry.important_name_of(ACTOR), Some(IMPORTANT_START));
    assert_eq!(registry.names_of(ACTOR), [None, Some(IMPORTANT_START)]);
    assert_eq!(registry.important_name_of(ACTOR + 1), None);
}

#[test]
fn a_talk_that_is_not_listed_is_found_but_not_important() {
    let mut registry = TalkRegistry::default();
    registry.register_name(ACTOR, 0, ORDINARY_START);
    assert_eq!(registry.names_of(ACTOR), [Some(ORDINARY_START), None]);
    assert_eq!(registry.important_name_of(ACTOR), None);
}

#[test]
fn registering_again_overwrites_that_mode_only() {
    let mut registry = TalkRegistry::default();
    registry.register_name(ACTOR, 0, ORDINARY_START);
    registry.register_name(ACTOR, 1, ORDINARY_START);
    registry.register_name(ACTOR, 0, IMPORTANT_START);
    assert_eq!(
        registry.names_of(ACTOR),
        [Some(IMPORTANT_START), Some(ORDINARY_START)]
    );
}

#[test]
fn an_empty_name_or_an_unknown_mode_registers_nothing() {
    let mut registry = TalkRegistry::default();
    registry.register_name(ACTOR, 0, "");
    registry.register_name(ACTOR, 2, IMPORTANT_START);
    registry.register_name(ACTOR, -1, IMPORTANT_START);
    assert_eq!(registry.names_of(ACTOR), [None, None]);
}

#[test]
fn a_new_phase_clears_the_registry() {
    let mut registry = TalkRegistry::default();
    registry.enter_phase("A_CENTER_VILLAGE_01");
    registry.register_name(ACTOR, 0, IMPORTANT_START);
    registry.enter_phase("A_CENTER_VILLAGE_01");
    assert!(registry.important_name_of(ACTOR).is_some());
    registry.enter_phase("A_CENTER_LIBRARY_01");
    assert_eq!(registry.important_name_of(ACTOR), None);
}

#[test]
fn a_press_is_logged_once_per_actor_until_the_phase_changes() {
    let mut registry = TalkRegistry::default();
    registry.enter_phase("A_CENTER_VILLAGE_01");
    assert!(registry.first_press(ACTOR));
    assert!(!registry.first_press(ACTOR));
    assert!(registry.first_press(ACTOR + 1));
    registry.enter_phase("A_CENTER_LIBRARY_01");
    assert!(registry.first_press(ACTOR));
}

#[test]
fn the_press_line_names_both_modes_and_the_talk_id() {
    let mut registry = TalkRegistry::default();
    registry.register_name(ACTOR, 0, IMPORTANT_START);
    registry.register_talk_id(ACTOR, 1, 42);
    assert_eq!(
        registry.press_line(ACTOR),
        "talk press: actor 7 mode0 EID_0100_a0038g_Start mode1 none talk id 42"
    );
    assert_eq!(
        registry.press_line(ACTOR + 1),
        "talk press: actor 8 mode0 none mode1 none talk id none"
    );
}

#[test]
fn a_numbered_talk_maps_to_its_important_base() {
    assert_eq!(important_base_of_talk_id(1031), Some("EID_0100_a0038g"));
    assert_eq!(important_base_of_talk_id(1014), Some("EID_0100_a0020"));
}

#[test]
fn a_talk_id_that_is_not_listed_is_not_important() {
    assert_eq!(important_base_of_talk_id(-5), None);
    assert_eq!(important_base_of_talk_id(1032), None);
}

#[test]
fn the_generated_id_table_is_sorted_and_free_of_duplicates() {
    assert!(IMPORTANT_TALK_IDS.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn a_registered_talk_id_is_refused_when_the_partner_is_far() {
    let mut registry = TalkRegistry::default();
    registry.register_talk_id(ACTOR, 0, 1031);
    assert_eq!(
        registry.important_talk_id_of(ACTOR),
        Some((1031, "EID_0100_a0038g"))
    );
    let far = partner_at(500.0, PHASE, 0);
    assert!(matches!(
        verdict_with(true, true, Some(&far)),
        PressVerdict::Wait { .. }
    ));
}

#[test]
fn a_registered_talk_id_that_is_not_listed_passes() {
    let mut registry = TalkRegistry::default();
    registry.register_talk_id(ACTOR, 0, 1032);
    assert_eq!(registry.important_talk_id_of(ACTOR), None);
}
