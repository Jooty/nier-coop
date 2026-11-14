use replicant_game::own_ride::{
    DROPPED_RIDE_STEP_SETTLES_AFTER_MS, DROPPED_RIDE_STEP_WATCHED_FOR_MS, DroppedRideStep,
    DroppedStepRelease, DroppedStepView,
};

const SECOND_RIDE_STEP: &str = "EID_1410_a0100g_21_04";
const SCRIPT_PAUSE_OF_THE_STEP_BEFORE: u32 = 0x80;
const GUEST_DROPPED_THE_STEP_MS: u64 = 32_989;
const HOST_TURN_STEP_RAN_HERE_MS: u64 = 35_552;
const GUEST_PAUSE_TAKEN_OFF_BY_A_LATER_STEP_MS: u64 = 89_852;

fn paused_with_nothing_running() -> DroppedStepView {
    DroppedStepView {
        slot_zero: SCRIPT_PAUSE_OF_THE_STEP_BEFORE,
        script_slot_idle: true,
        partner_calls_clear: true,
        something_open: false,
        in_play: true,
    }
}

fn dropped_in_the_run() -> DroppedRideStep {
    let mut dropped = DroppedRideStep::new();
    dropped.note_dropped(SECOND_RIDE_STEP, GUEST_DROPPED_THE_STEP_MS);
    dropped.note_step(HOST_TURN_STEP_RAN_HERE_MS);
    dropped
}

#[test]
fn the_pause_the_dropped_step_left_is_taken_off_once_the_steps_settle() {
    let mut dropped = dropped_in_the_run();
    let settled_ms = HOST_TURN_STEP_RAN_HERE_MS + DROPPED_RIDE_STEP_SETTLES_AFTER_MS;
    assert_eq!(
        dropped.observe(paused_with_nothing_running(), settled_ms - 1),
        None
    );
    assert_eq!(
        dropped.observe(paused_with_nothing_running(), settled_ms),
        Some(DroppedStepRelease {
            step: SECOND_RIDE_STEP.to_owned(),
            mask: SCRIPT_PAUSE_OF_THE_STEP_BEFORE,
            after_ms: settled_ms - GUEST_DROPPED_THE_STEP_MS,
        })
    );
    assert!(!dropped.is_watching());
    assert!(settled_ms < GUEST_PAUSE_TAKEN_OFF_BY_A_LATER_STEP_MS);
}

#[test]
fn a_talk_menu_or_scene_keeps_the_pause_until_it_closes() {
    let mut dropped = dropped_in_the_run();
    let settled_ms = HOST_TURN_STEP_RAN_HERE_MS + DROPPED_RIDE_STEP_SETTLES_AFTER_MS;
    let talk_up = DroppedStepView {
        something_open: true,
        ..paused_with_nothing_running()
    };
    assert_eq!(dropped.observe(talk_up, settled_ms), None);
    assert!(
        dropped
            .observe(paused_with_nothing_running(), settled_ms + 500)
            .is_some()
    );
}

#[test]
fn a_running_script_or_a_queued_partner_call_keeps_the_pause() {
    let mut dropped = dropped_in_the_run();
    let settled_ms = HOST_TURN_STEP_RAN_HERE_MS + DROPPED_RIDE_STEP_SETTLES_AFTER_MS;
    let script_running = DroppedStepView {
        script_slot_idle: false,
        ..paused_with_nothing_running()
    };
    let call_queued = DroppedStepView {
        partner_calls_clear: false,
        ..paused_with_nothing_running()
    };
    let loading = DroppedStepView {
        in_play: false,
        ..paused_with_nothing_running()
    };
    assert_eq!(dropped.observe(script_running, settled_ms), None);
    assert_eq!(dropped.observe(call_queued, settled_ms), None);
    assert_eq!(dropped.observe(loading, settled_ms), None);
    assert!(dropped.is_watching());
}

#[test]
fn a_partner_step_arriving_later_restarts_the_settle() {
    let mut dropped = dropped_in_the_run();
    let late_arrival_ms = HOST_TURN_STEP_RAN_HERE_MS + 2_000;
    dropped.note_step(late_arrival_ms);
    assert_eq!(
        dropped.observe(
            paused_with_nothing_running(),
            HOST_TURN_STEP_RAN_HERE_MS + DROPPED_RIDE_STEP_SETTLES_AFTER_MS
        ),
        None
    );
    assert!(
        dropped
            .observe(
                paused_with_nothing_running(),
                late_arrival_ms + DROPPED_RIDE_STEP_SETTLES_AFTER_MS
            )
            .is_some()
    );
}

#[test]
fn a_pause_the_game_took_off_itself_is_never_touched_again() {
    let mut dropped = dropped_in_the_run();
    let unpaused = DroppedStepView {
        slot_zero: 0,
        ..paused_with_nothing_running()
    };
    assert_eq!(dropped.observe(unpaused, 36_000), None);
    assert!(!dropped.is_watching());
    assert_eq!(
        dropped.observe(
            paused_with_nothing_running(),
            HOST_TURN_STEP_RAN_HERE_MS + DROPPED_RIDE_STEP_SETTLES_AFTER_MS
        ),
        None
    );
}

#[test]
fn this_players_own_ride_keeps_its_own_pause() {
    let mut dropped = dropped_in_the_run();
    dropped.forget();
    assert_eq!(
        dropped.observe(
            paused_with_nothing_running(),
            HOST_TURN_STEP_RAN_HERE_MS + DROPPED_RIDE_STEP_SETTLES_AFTER_MS
        ),
        None
    );
}

#[test]
fn the_watch_ends_after_its_time() {
    let mut dropped = DroppedRideStep::new();
    dropped.note_dropped(SECOND_RIDE_STEP, GUEST_DROPPED_THE_STEP_MS);
    assert_eq!(
        dropped.observe(
            paused_with_nothing_running(),
            GUEST_DROPPED_THE_STEP_MS + DROPPED_RIDE_STEP_WATCHED_FOR_MS + 1
        ),
        None
    );
    assert!(!dropped.is_watching());
}
