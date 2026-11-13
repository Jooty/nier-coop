use automata_game::partner::{
    OwnDashCut, buttons_let_go, is_a_dash_state, own_dash_cut_line, start_cuts_a_dash,
};

const DASH: i32 = 0x12;
const SPRINT: i32 = 0x3;
const RUN: i32 = 0x2;
const IDLE: i32 = 0x0;
const DASH_BUTTON_BIT: u32 = 0x80;
const NO_BUTTONS_HELD: u32 = 0x0;

fn cut(held_now: u32, held_at_start: Option<u32>) -> OwnDashCut {
    OwnDashCut {
        from_state: SPRINT,
        to_state: RUN,
        held_buttons_now: held_now,
        held_buttons_at_the_dash_start: held_at_start,
        speed: 1000.0,
        input_ignore_seconds: 0.0,
        inside_a_held_body_update: false,
        caller_rvas: vec![0x42_DEF4, 0x43_0D0F],
    }
}

#[test]
fn the_dash_and_the_sprint_are_the_dash_states() {
    assert!(is_a_dash_state(DASH));
    assert!(is_a_dash_state(SPRINT));
    assert!(!is_a_dash_state(RUN));
    assert!(!is_a_dash_state(IDLE));
}

#[test]
fn going_from_the_dash_into_the_sprint_is_no_cut() {
    assert!(!start_cuts_a_dash(DASH, SPRINT));
    assert!(!start_cuts_a_dash(SPRINT, DASH));
}

#[test]
fn leaving_the_sprint_for_a_run_is_a_cut() {
    assert!(start_cuts_a_dash(SPRINT, RUN));
    assert!(start_cuts_a_dash(DASH, IDLE));
}

#[test]
fn a_start_out_of_a_plain_state_is_no_cut() {
    assert!(!start_cuts_a_dash(RUN, IDLE));
    assert!(!start_cuts_a_dash(IDLE, DASH));
}

#[test]
fn a_dash_button_still_held_counts_as_nothing_let_go() {
    assert_eq!(
        buttons_let_go(&cut(DASH_BUTTON_BIT, Some(DASH_BUTTON_BIT))),
        Some(0)
    );
}

#[test]
fn a_released_dash_button_is_named_as_let_go() {
    assert_eq!(
        buttons_let_go(&cut(NO_BUTTONS_HELD, Some(DASH_BUTTON_BIT))),
        Some(DASH_BUTTON_BIT)
    );
}

#[test]
fn without_a_seen_dash_start_what_was_let_go_is_unknown() {
    assert_eq!(buttons_let_go(&cut(DASH_BUTTON_BIT, None)), None);
}

#[test]
fn the_line_names_the_states_the_buttons_and_the_callers() {
    let line = own_dash_cut_line(&cut(DASH_BUTTON_BIT, Some(DASH_BUTTON_BIT)), 3, 40);
    assert!(line.starts_with("own dash: the hero went from state 0x3 to 0x2;"));
    assert!(line.contains("held buttons 0x80 (at the dash's start 0x80, let go since: none)"));
    assert!(line.contains("a held body's update was running: no"));
    assert!(line.contains("game code on the stack 0x42DEF4, 0x430D0F"));
    assert!(line.ends_with("(line 3 of at most 40)"));
}
