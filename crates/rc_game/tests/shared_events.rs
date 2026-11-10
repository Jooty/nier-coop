use rc_game::callback_gate::event_base;
use rc_game::shared_events::{
    SHARED_EVENT_BASES, SharedEventsRan, SharedFire, StartPath, is_shared_event, phase_name,
    start_path,
};

const NORTH_FIELD: &str = "A_NORTH_FIELD_01";
const VILLAGE: &str = "A_CENTER_VILLAGE_01";
const AMBUSH_START: &str = "SEID_0100_S0010_a0050_Start";
const AMBUSH_BASE: &str = "SEID_0100_S0010_a0050";
const NORTH_PHASE: u16 = 7;
const OTHER_PHASE: u16 = 8;

#[test]
fn the_table_is_sorted_so_the_lookup_can_search_it() {
    assert!(SHARED_EVENT_BASES.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn the_north_ambush_is_in_the_table_and_the_village_talk_is_not() {
    assert!(is_shared_event(NORTH_FIELD, AMBUSH_START));
    assert!(!is_shared_event(VILLAGE, AMBUSH_START));
    assert!(SHARED_EVENT_BASES.contains(&(NORTH_FIELD, AMBUSH_BASE)));
    assert!(!SHARED_EVENT_BASES.contains(&(VILLAGE, AMBUSH_BASE)));
}

#[test]
fn only_a_start_callback_counts_as_a_shared_event() {
    assert!(!is_shared_event(NORTH_FIELD, "SEID_0100_S0010_a0050_End"));
    assert!(!is_shared_event(NORTH_FIELD, "Boss_Battle_Start"));
}

#[test]
fn a_shared_start_skips_the_local_scene_talk_rule() {
    assert_eq!(
        start_path(Some(NORTH_FIELD), AMBUSH_START, false),
        StartPath::SharedEvent
    );
    assert_eq!(
        start_path(Some(VILLAGE), AMBUSH_START, false),
        StartPath::SceneTalk
    );
    assert_eq!(start_path(None, AMBUSH_START, false), StartPath::SceneTalk);
}

#[test]
fn a_talk_you_pressed_stays_yours_and_plain_events_take_the_cutscene_path() {
    assert_eq!(
        start_path(Some(NORTH_FIELD), AMBUSH_START, true),
        StartPath::YourTalk
    );
    assert_eq!(
        start_path(Some(VILLAGE), "EID_0100_zzzz_Start", false),
        StartPath::Cutscene
    );
}

#[test]
fn the_phase_index_names_its_pack() {
    assert!(phase_name(u16::MAX).is_none());
    assert!(phase_name(0).is_some());
}

#[test]
fn a_fire_after_the_scene_ran_in_this_phase_is_dropped_once_logged() {
    let mut ran = SharedEventsRan::new();
    assert_eq!(ran.judge(NORTH_PHASE, AMBUSH_BASE, false), SharedFire::Run);
    ran.note_ran(NORTH_PHASE, AMBUSH_BASE);
    assert_eq!(
        ran.judge(NORTH_PHASE, AMBUSH_BASE, false),
        SharedFire::Drop {
            first_for_base: true
        }
    );
    assert_eq!(
        ran.judge(NORTH_PHASE, AMBUSH_BASE, false),
        SharedFire::Drop {
            first_for_base: false
        }
    );
}

#[test]
fn a_fire_the_partner_led_still_runs_after_it_was_noted() {
    let mut ran = SharedEventsRan::new();
    ran.note_ran(NORTH_PHASE, AMBUSH_BASE);
    assert_eq!(ran.judge(NORTH_PHASE, AMBUSH_BASE, true), SharedFire::Run);
}

#[test]
fn a_new_phase_forgets_what_ran() {
    let mut ran = SharedEventsRan::new();
    ran.note_ran(NORTH_PHASE, AMBUSH_BASE);
    assert_eq!(ran.judge(OTHER_PHASE, AMBUSH_BASE, false), SharedFire::Run);
    ran.note_ran(OTHER_PHASE, "EID_0100_zzzz");
    assert_eq!(ran.judge(NORTH_PHASE, AMBUSH_BASE, false), SharedFire::Run);
}

#[test]
fn the_start_name_reduces_to_the_table_base() {
    assert_eq!(event_base(AMBUSH_START), AMBUSH_BASE);
}

#[test]
fn a_fire_is_partner_led_only_after_the_partner_fired_it() {
    let mut gate = rc_game::callback_gate::CallbackGate::new();
    assert!(!gate.partner_led(AMBUSH_START, 0));
    gate.partner_fire(rc_game::callback_gate::GateRole::Guest, AMBUSH_START, 0);
    assert!(gate.partner_led(AMBUSH_START, 1));
}
