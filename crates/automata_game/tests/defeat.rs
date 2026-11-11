use automata_game::address::{
    DEFEAT_FLOW_STATE_VALUE, DEFEAT_FLOW_STEP_VALUE, RESTORE_POINT_POINTER_VALUE,
};
use automata_game::defeat::{
    DEFEAT_FLOW_LINES_LOGGED_AT_MOST, DEFEAT_FLOW_STALL_LINE_INTERVAL_MS, DefeatFlow,
    DefeatFlowLog, HealthLog, HoldLog, OtherPlayerHold, PLAYER_DAMAGE_FUNCTION_START,
    PLAYER_DAMAGE_VTABLE_OFFSET, RestorePoint, health_kept_on_the_partner_body, hold_cause,
    is_hero_defeat_state, local_game_may_start_on_the_held_body,
};
use automata_game::exe::ExeVariant;
use automata_game::partner::NotFree;

const IDLE: DefeatFlow = DefeatFlow { state: 0, step: 0 };

fn flow(state: i32, step: i32) -> DefeatFlow {
    DefeatFlow { state, step }
}

fn other(free: bool, health: i32, state: u16) -> Option<OtherPlayerHold> {
    Some(OtherPlayerHold {
        has_a_hero: true,
        free,
        health,
        state,
    })
}

#[test]
fn the_defeat_flow_values_sit_side_by_side_on_both_exes() {
    for variant in [ExeVariant::Main, ExeVariant::Compat] {
        assert_eq!(
            DEFEAT_FLOW_STEP_VALUE.rva(variant),
            DEFEAT_FLOW_STATE_VALUE.rva(variant) + 4
        );
    }
    assert_eq!(DEFEAT_FLOW_STATE_VALUE.rva(ExeVariant::Main), 0x11E_AC1C);
    assert_eq!(DEFEAT_FLOW_STATE_VALUE.rva(ExeVariant::Compat), 0x11E_0D4C);
    assert_eq!(
        RESTORE_POINT_POINTER_VALUE.rva(ExeVariant::Main),
        0x142_2278
    );
    assert_eq!(
        RESTORE_POINT_POINTER_VALUE.rva(ExeVariant::Compat),
        0x141_60F8
    );
}

#[test]
fn only_the_two_defeat_states_count_as_the_heros_defeat() {
    assert!(is_hero_defeat_state(0x35));
    assert!(is_hero_defeat_state(0x36));
    assert!(!is_hero_defeat_state(0));
    assert!(!is_hero_defeat_state(0x37));
}

#[test]
fn the_flow_is_active_in_every_state_but_idle() {
    assert!(!IDLE.is_active());
    for state in 1..=8 {
        assert!(flow(state, 0).is_active());
    }
    assert!(flow(7, 0).asks_for_the_title());
    assert!(!flow(3, 10).asks_for_the_title());
}

#[test]
fn every_step_of_the_usual_flow_has_its_own_words() {
    let steps = [0, 1, 10, 11, 12].map(|step| flow(3, step).describe());
    for (index, text) in steps.iter().enumerate() {
        assert!(text.starts_with("state 3"));
        assert!(steps.iter().skip(index + 1).all(|later| later != text));
    }
    assert!(flow(3, 10).describe().contains("restore point"));
    assert!(flow(5, 2).describe().contains("state 5"));
}

#[test]
fn a_flow_that_never_starts_logs_nothing() {
    let mut log = DefeatFlowLog::default();
    for now_ms in [0, 20_000, 40_000] {
        assert!(
            log.observe(now_ms, IDLE, Some(0), RestorePoint::Absent)
                .is_empty()
        );
    }
    assert_eq!(log.defeats(), 0);
}

#[test]
fn one_defeat_is_told_from_the_defeat_state_to_the_end_of_the_flow() {
    let mut log = DefeatFlowLog::default();
    assert!(
        log.observe(0, IDLE, Some(2), RestorePoint::Present)
            .is_empty()
    );
    let state = log.observe(100, IDLE, Some(0x35), RestorePoint::Present);
    assert_eq!(state, ["defeat: the hero is in its defeat state 0x35"]);
    let start = log.observe(3_600, flow(1, 0), Some(0x36), RestorePoint::Present);
    assert_eq!(start.len(), 2);
    assert_eq!(start[0], "defeat: the hero is in its defeat state 0x36");
    assert!(start[1].contains("starts (defeat 1 of this session"));
    assert!(start[1].contains("holds a restore point"));
    let step = log.observe(3_700, flow(3, 0), Some(0x36), RestorePoint::Present);
    assert_eq!(step.len(), 1);
    assert!(step[0].starts_with("defeat: flow: state 3"));
    assert!(
        log.observe(3_800, flow(3, 0), Some(0x36), RestorePoint::Present)
            .is_empty()
    );
    let gone = log.observe(5_000, flow(3, 11), None, RestorePoint::Present);
    assert!(gone[0].contains("is gone"));
    let over = log.observe(9_600, IDLE, Some(0), RestorePoint::Present);
    assert_eq!(
        over,
        ["defeat: the game's defeat flow is over after 6000 ms"]
    );
    assert_eq!(log.defeats(), 1);
}

#[test]
fn a_flow_that_does_not_move_says_so_every_ten_seconds() {
    let mut log = DefeatFlowLog::default();
    log.observe(0, flow(3, 1), None, RestorePoint::Absent);
    assert!(
        log.observe(
            DEFEAT_FLOW_STALL_LINE_INTERVAL_MS - 1,
            flow(3, 1),
            None,
            RestorePoint::Absent
        )
        .is_empty()
    );
    let stalled = log.observe(
        DEFEAT_FLOW_STALL_LINE_INTERVAL_MS,
        flow(3, 1),
        None,
        RestorePoint::Absent,
    );
    assert_eq!(stalled.len(), 1);
    assert!(stalled[0].contains("has not moved for 10 s"));
    assert!(stalled[0].contains("body left behind"));
    assert!(
        log.observe(
            DEFEAT_FLOW_STALL_LINE_INTERVAL_MS + 5_000,
            flow(3, 1),
            None,
            RestorePoint::Absent
        )
        .is_empty()
    );
}

#[test]
fn the_flow_log_stops_at_its_line_limit() {
    let mut log = DefeatFlowLog::default();
    let mut lines = 0;
    for tick in 0..(u64::from(DEFEAT_FLOW_LINES_LOGGED_AT_MOST) * 2) {
        lines += log
            .observe(tick, flow(3, tick as i32), None, RestorePoint::Absent)
            .len();
    }
    assert_eq!(lines, DEFEAT_FLOW_LINES_LOGGED_AT_MOST as usize);
}

#[test]
fn health_lines_are_the_first_reading_big_steps_defeat_and_return() {
    let mut log = HealthLog::of("the hero");
    assert_eq!(
        log.observe(800, 800).as_deref(),
        Some("health: the hero has 800 of 800")
    );
    assert_eq!(log.observe(790, 800), None);
    assert_eq!(log.observe(730, 800), None);
    assert_eq!(
        log.observe(700, 800).as_deref(),
        Some("health: the hero lost 100 and has 700 of 800")
    );
    assert_eq!(
        log.observe(0, 800).as_deref(),
        Some("health: the hero has no health left: defeated (it had 700)")
    );
    assert_eq!(log.observe(0, 800), None);
    assert_eq!(
        log.observe(800, 800).as_deref(),
        Some("health: the hero is back with 800 of 800")
    );
    assert_eq!(
        log.observe(800, 900).as_deref(),
        Some("health: the most the hero can have went from 800 to 900")
    );
}

#[test]
fn a_small_last_hit_still_logs_the_defeat() {
    let mut log = HealthLog::of("the other player");
    log.observe(5, 800);
    assert!(
        log.observe(0, 800)
            .is_some_and(|line| line.contains("defeated"))
    );
}

#[test]
fn a_hold_is_logged_at_its_start_at_each_new_cause_and_at_its_end() {
    let mut log = HoldLog::default();
    assert_eq!(log.observe(0, true, "x".to_owned()), None);
    assert_eq!(
        log.observe(10, false, "a".to_owned()).as_deref(),
        Some("hold: starts: a")
    );
    assert_eq!(log.observe(20, false, "a".to_owned()), None);
    assert_eq!(
        log.observe(30, false, "b".to_owned()).as_deref(),
        Some("hold: goes on, now: b")
    );
    assert_eq!(
        log.observe(1_010, true, "ignored".to_owned()).as_deref(),
        Some("hold: ends after 1000 ms, both players are free (the last cause: b)")
    );
    assert_eq!(log.observe(1_020, true, "ignored".to_owned()), None);
}

#[test]
fn the_cause_of_a_hold_names_the_player_and_a_defeat() {
    let local = hold_cause(Some(NotFree::SceneInputOffBit), true, other(true, 800, 0));
    assert!(local.starts_with("the local player: the scene input-off bit"));
    assert!(local.ends_with("(the game's defeat flow runs)"));
    let defeated = hold_cause(None, false, other(false, 0, 0x35));
    assert!(defeated.contains("after a defeat (health 0, state 0x35)"));
    let scene = hold_cause(None, false, other(false, 640, 0));
    assert!(scene.starts_with("the other player: its game holds it (a cutscene"));
    let both = hold_cause(Some(NotFree::NotInPlay), false, other(false, 0, 0x36));
    assert!(both.contains("the local player: the scene is not in play; the other player:"));
    assert!(hold_cause(None, false, None).contains("no state heard"));
    let loading = Some(OtherPlayerHold {
        has_a_hero: false,
        free: false,
        health: 0,
        state: 0,
    });
    assert!(hold_cause(None, false, loading).contains("has no hero in play"));
    assert!(!hold_cause(None, false, loading).contains("defeat"));
    assert!(hold_cause(None, false, other(true, 800, 0)).contains("not yet for long enough"));
}

#[test]
fn the_partner_body_shows_the_owners_health_but_never_less_than_one() {
    assert_eq!(health_kept_on_the_partner_body(430, 800), 430);
    assert_eq!(health_kept_on_the_partner_body(0, 800), 1);
    assert_eq!(health_kept_on_the_partner_body(-5, 800), 1);
    assert_eq!(health_kept_on_the_partner_body(1_200, 800), 800);
    assert_eq!(health_kept_on_the_partner_body(500, 0), 1);
}

#[test]
fn this_game_starts_any_state_on_the_held_body_but_a_defeat_state() {
    assert!(!local_game_may_start_on_the_held_body(0x35));
    assert!(!local_game_may_start_on_the_held_body(0x36));
    for state in [0, 2, 0x2A, 0x2B, 0x37, 0xBF] {
        assert!(local_game_may_start_on_the_held_body(state));
    }
}

#[test]
fn the_damage_function_is_the_noted_slot_on_both_exes() {
    use automata_game::address::PLAYER_DAMAGE_FUNCTION;
    assert_eq!(PLAYER_DAMAGE_FUNCTION.rva(ExeVariant::Main), 0x43_4530);
    assert_eq!(PLAYER_DAMAGE_FUNCTION.rva(ExeVariant::Compat), 0x44_6DF0);
    assert_eq!(PLAYER_DAMAGE_VTABLE_OFFSET, 151 * 8);
    assert_eq!(PLAYER_DAMAGE_FUNCTION_START.len(), 28);
}
