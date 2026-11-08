use rc_game::script_runner::{
    PauseCall, ScriptRunnerLogGate, ScriptRunnerSnapshot, format_empty_pop, format_pause_call,
    format_runner_state,
};

fn snapshot(pause_mask: u32, queued: u32) -> ScriptRunnerSnapshot {
    ScriptRunnerSnapshot {
        pause_mask,
        queued,
        ready: queued,
        last_pop: Some(("EID_0100_a0038g_End".to_owned(), 12_500)),
    }
}

#[test]
fn the_state_line_names_pause_queue_ready_and_the_last_pop() {
    assert_eq!(
        format_runner_state(&snapshot(0x7f, 2)),
        "scripts: pause 0x7f, queue 2, ready 2, last pop EID_0100_a0038g_End 12s ago"
    );
}

#[test]
fn a_state_line_without_any_pop_says_so() {
    let mut view = snapshot(0, 1);
    view.last_pop = None;
    assert!(format_runner_state(&view).ends_with("no pop yet"));
}

#[test]
fn the_empty_pop_line_names_what_is_queued() {
    assert_eq!(
        format_empty_pop(&snapshot(0, 3)),
        "scripts: the game's pop returned nothing while 3 queued (3 ready), pause 0x0, last pop EID_0100_a0038g_End 12s ago"
    );
}

#[test]
fn the_pause_line_shows_the_call_the_script_and_the_mask_change() {
    assert_eq!(
        format_pause_call(PauseCall::On, 0, 0x7f, "EID_0100_a0038g_End"),
        "scripts: PauseOn by EID_0100_a0038g_End (pause 0x0 -> 0x7f)"
    );
    assert_eq!(
        format_pause_call(PauseCall::Off, 0x7f, 0, ""),
        "scripts: PauseOff by unknown (pause 0x7f -> 0x0)"
    );
}

#[test]
fn an_idle_runner_is_never_logged() {
    assert!(snapshot(0, 0).is_idle());
    assert!(!snapshot(4, 0).is_idle());
    assert!(!snapshot(0, 1).is_idle());
    let mut gate = ScriptRunnerLogGate::new();
    assert!(!gate.state_due(true, 5_000));
}

#[test]
fn a_busy_runner_is_logged_once_a_second() {
    let mut gate = ScriptRunnerLogGate::new();
    assert!(gate.state_due(false, 10_000));
    assert!(!gate.state_due(false, 10_400));
    assert!(gate.state_due(false, 11_000));
}

#[test]
fn going_idle_makes_the_next_busy_moment_log_at_once() {
    let mut gate = ScriptRunnerLogGate::new();
    assert!(gate.state_due(false, 10_000));
    assert!(!gate.state_due(true, 10_100));
    assert!(gate.state_due(false, 10_200));
}

#[test]
fn empty_pops_are_logged_at_most_once_a_second() {
    let mut gate = ScriptRunnerLogGate::new();
    assert!(gate.empty_pop_due(1_000));
    assert!(!gate.empty_pop_due(1_999));
    assert!(gate.empty_pop_due(2_000));
}
