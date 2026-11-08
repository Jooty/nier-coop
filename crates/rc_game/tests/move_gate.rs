use rc_game::move_gate::{
    MOVE_HOLD_GIVE_UP_MS, MOVE_HOLD_REMINDER_MS, MOVING_EVENT_BASES, MoveAsk, MoveHolds,
    MoveRelease, MoveVerdict, StartAnswer, judge_move, moves_the_player,
};
use rc_game::stay_together::PARTNER_TIMEOUT_MS;
use rc_game::zone_rules::{Companion, Placement, ZoneRadius};

const MOVING_START: &str = "EID_0100_a0038g_Start";
const MOVING_END: &str = "EID_0100_a0038g_End";
const PHASE: u16 = 3;
const LOCAL: Placement = Placement {
    phase: PHASE,
    position: [0.0, 0.0, 0.0],
};

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

fn verdict_with(connected: bool, game_open: bool, partner: Option<&Companion>) -> MoveVerdict {
    judge_move(&MoveAsk {
        connected,
        partner_game_open: game_open,
        local: Some(LOCAL),
        partner,
        radius: ZoneRadius::default(),
    })
}

fn far() -> MoveVerdict {
    verdict_with(true, true, Some(&partner_at(100.0, PHASE, 0)))
}

#[test]
fn the_table_holds_the_next_day_cutscene() {
    assert!(MOVING_EVENT_BASES.contains(&"EID_0100_a0038g"));
    assert!(moves_the_player(MOVING_START));
    assert!(moves_the_player(MOVING_END));
}

#[test]
fn the_table_is_sorted_and_free_of_duplicates() {
    assert!(MOVING_EVENT_BASES.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn a_scene_that_is_not_in_the_table_does_not_move_the_player() {
    assert!(!moves_the_player("EID_0100_zzzz_Start"));
    assert!(!moves_the_player("CB_Free_Camera_01"));
    assert!(!moves_the_player("EID_0100_a0038g"));
}

#[test]
fn a_near_partner_lets_the_scene_run() {
    let near = partner_at(5.0, PHASE, 0);
    assert_eq!(verdict_with(true, true, Some(&near)), MoveVerdict::Allow);
}

#[test]
fn a_far_partner_makes_the_scene_wait() {
    let MoveVerdict::Wait {
        partner,
        distance_units,
    } = far()
    else {
        panic!("expected a wait");
    };
    assert_eq!(partner, "Sam");
    assert!((distance_units - 100.0).abs() < 0.01);
}

#[test]
fn the_wait_text_names_the_partner() {
    let text = far().prompt_text().unwrap();
    assert!(text.contains("Sam"));
    assert_eq!(MoveVerdict::Allow.prompt_text(), None);
}

#[test]
fn a_partner_who_is_away_never_blocks() {
    let far_partner = partner_at(100.0, PHASE, 0);
    assert_eq!(
        verdict_with(false, true, Some(&far_partner)),
        MoveVerdict::Allow
    );
    assert_eq!(
        verdict_with(true, false, Some(&far_partner)),
        MoveVerdict::Allow
    );
    assert_eq!(verdict_with(true, true, None), MoveVerdict::Allow);
    let other_phase = partner_at(100.0, PHASE + 1, 0);
    assert_eq!(
        verdict_with(true, true, Some(&other_phase)),
        MoveVerdict::Allow
    );
    let stale = partner_at(100.0, PHASE, PARTNER_TIMEOUT_MS + 1);
    assert_eq!(verdict_with(true, true, Some(&stale)), MoveVerdict::Allow);
}

#[test]
fn a_far_start_is_held_once_then_kept_held() {
    let mut holds = MoveHolds::new();
    assert_eq!(
        holds.on_start(MOVING_START, PHASE, true, &far(), 0),
        StartAnswer::Hold { first: true }
    );
    assert_eq!(
        holds.on_start(MOVING_START, PHASE, true, &far(), 100),
        StartAnswer::Hold { first: false }
    );
    assert!(holds.is_holding());
}

#[test]
fn an_allowed_start_runs_at_once() {
    let mut holds = MoveHolds::new();
    assert_eq!(
        holds.on_start(MOVING_START, PHASE, false, &MoveVerdict::Allow, 0),
        StartAnswer::Run
    );
    assert!(!holds.is_holding());
}

#[test]
fn the_held_start_is_released_when_the_partner_arrives_and_then_passes_once() {
    let mut holds = MoveHolds::new();
    holds.on_start(MOVING_START, PHASE, true, &far(), 0);
    assert!(holds.step(&far(), Some(PHASE), 1000).is_empty());
    let released = holds.step(&MoveVerdict::Allow, Some(PHASE), 2000);
    assert_eq!(released.len(), 1);
    assert_eq!(released[0].why, MoveRelease::PartnerArrived);
    assert!(released[0].held.after_talk_command);
    assert_eq!(
        holds.on_start(MOVING_START, PHASE, true, &far(), 2100),
        StartAnswer::Run
    );
    assert_eq!(
        holds.on_start(MOVING_START, PHASE, true, &far(), 2200),
        StartAnswer::Hold { first: true }
    );
}

#[test]
fn a_start_held_too_long_gives_up_and_runs() {
    let mut holds = MoveHolds::new();
    holds.on_start(MOVING_START, PHASE, false, &far(), 0);
    let released = holds.step(&far(), Some(PHASE), MOVE_HOLD_GIVE_UP_MS);
    assert_eq!(released.len(), 1);
    assert_eq!(
        released[0].why,
        MoveRelease::GaveUp {
            waited_ms: MOVE_HOLD_GIVE_UP_MS
        }
    );
}

#[test]
fn leaving_the_area_drops_the_hold_without_a_pass() {
    let mut holds = MoveHolds::new();
    holds.on_start(MOVING_START, PHASE, false, &far(), 0);
    let released = holds.step(&far(), Some(PHASE + 1), 500);
    assert_eq!(released[0].why, MoveRelease::LeftPhase);
    assert_eq!(
        holds.on_start(MOVING_START, PHASE, false, &far(), 600),
        StartAnswer::Hold { first: true }
    );
}

#[test]
fn the_reminder_comes_every_few_seconds_while_held() {
    let mut holds = MoveHolds::new();
    assert!(!holds.reminder_due(10_000));
    holds.on_start(MOVING_START, PHASE, false, &far(), 0);
    assert!(!holds.reminder_due(MOVE_HOLD_REMINDER_MS - 1));
    assert!(holds.reminder_due(MOVE_HOLD_REMINDER_MS));
    assert!(!holds.reminder_due(MOVE_HOLD_REMINDER_MS + 1));
}
