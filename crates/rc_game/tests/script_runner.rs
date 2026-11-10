use rc_game::fight_sync::SlotState;
use rc_game::script_runner::{
    PauseCall, SLOT_HOLDER_LOGGED_EVERY_MS, ScriptRunnerLogGate, ScriptRunnerSnapshot,
    SlotHolderReport, SlotHolderWatch, format_empty_pop, format_pause_call, format_runner_state,
    format_slot_holder,
};

const STUCK_TALK: &str = "EID_0200_a0040g_Talk";

fn snapshot(pause_mask: u32, queued: u32) -> ScriptRunnerSnapshot {
    ScriptRunnerSnapshot {
        pause_mask,
        queued,
        ready: queued,
        last_pop: Some(("EID_0100_a0038g_End".to_owned(), 12_500)),
        slot: SlotState::Busy,
        manager_state: Some(0x6B0),
        last_native: Some(("_StartTalk", 300)),
    }
}

#[test]
fn the_state_line_names_pause_queue_ready_the_slot_the_last_pop_and_its_last_native() {
    assert_eq!(
        format_runner_state(&snapshot(0x7f, 2)),
        "scripts: pause 0x7f, queue 2, ready 2, slot Busy (state 0x6b0), last pop EID_0100_a0038g_End 12s ago, last native _StartTalk 300 ms ago"
    );
}

#[test]
fn a_handler_holding_the_slot_is_reported_once_per_five_seconds_until_the_slot_is_idle() {
    let every = SLOT_HOLDER_LOGGED_EVERY_MS;
    let mut watch = SlotHolderWatch::new();
    assert_eq!(watch.tick(SlotState::Busy, 0), None, "nothing popped yet");
    watch.popped(STUCK_TALK, 1_000);
    watch.native_called("_StartTalk", 1_200);
    assert_eq!(watch.tick(SlotState::Busy, 1_000 + every), None);
    let report = watch.tick(SlotState::SteppingHeld, 1_001 + every);
    assert_eq!(
        report,
        Some(SlotHolderReport {
            handler: STUCK_TALK.to_owned(),
            in_slot_ms: every + 1,
            last_native: Some(("_StartTalk", every - 199)),
            last_yielding_native: None,
        })
    );
    assert_eq!(
        format_slot_holder(&report.unwrap(), 0x19f, true),
        "scripts: EID_0200_a0040g_Talk in the slot 5001 ms, last native _StartTalk 4801 ms ago, no yielding native since the pop, pause 0x19f, menu open true"
    );
    assert_eq!(watch.tick(SlotState::Busy, 1_001 + 2 * every), None);
    assert!(watch.tick(SlotState::Busy, 1_002 + 2 * every).is_some());
    assert_eq!(watch.tick(SlotState::Idle, 1_003 + 2 * every), None);
    assert_eq!(
        watch.tick(SlotState::Busy, 60_000),
        None,
        "an idle slot ended that handler"
    );
    assert_eq!(watch.last_native(60_000), Some(("_StartTalk", 58_800)));
    watch.popped(STUCK_TALK, 70_000);
    assert_eq!(watch.last_native(70_000), None, "a new pop starts clean");
}

#[test]
fn a_per_frame_native_never_hides_the_native_the_holder_waits_in() {
    let mut watch = SlotHolderWatch::new();
    watch.popped(STUCK_TALK, 0);
    watch.yielding_native_called("_StartRadio", 100);
    watch.native_called("_GetActorLife", 200);
    for now_ms in (300..6_000).step_by(16) {
        watch.native_called("_DebugPrint", now_ms);
    }
    assert_eq!(watch.last_native(6_000), Some(("_GetActorLife", 5_800)));
    assert_eq!(
        watch.last_yielding_native(6_000),
        Some(("_StartRadio", 5_900))
    );
    assert_eq!(watch.holding_for(6_000), Some((STUCK_TALK, 6_000)));
    let report = watch.tick(SlotState::Busy, 6_000).unwrap();
    assert_eq!(
        format_slot_holder(&report, 0x9f, true),
        "scripts: EID_0200_a0040g_Talk in the slot 6000 ms, last native _GetActorLife 5800 ms ago, last yielding native _StartRadio 5900 ms ago, pause 0x9f, menu open true"
    );
    assert_eq!(watch.tick(SlotState::Idle, 6_100), None);
    assert_eq!(watch.holding_for(6_100), None, "an idle slot has no holder");
}

#[test]
fn a_state_line_without_any_pop_says_so() {
    let mut view = snapshot(0, 1);
    view.last_pop = None;
    view.last_native = None;
    view.manager_state = None;
    assert!(
        format_runner_state(&view)
            .ends_with("state unread), no pop yet, no hooked native since the pop")
    );
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
