use automata_game::load_gate::{FIRST_SYSTEM_PHASE_ID, LoadWatch, LoadWatchReadings};

fn readings(scene_state: i32, phase_id: i32, campaign_link_up: bool) -> LoadWatchReadings {
    LoadWatchReadings {
        scene_state: Some(scene_state),
        phase_id: Some(phase_id),
        campaign_link_up,
    }
}

fn running_watch() -> LoadWatch {
    let mut watch = LoadWatch::default();
    watch.step(&readings(1, 0x100, true));
    watch
}

#[test]
fn a_load_begins_when_the_scene_state_leaves_running_and_ends_when_it_returns() {
    let mut watch = running_watch();
    assert!(!watch.step(&readings(1, 0x100, true)).began);
    let began = watch.step(&readings(2, 0x100, true));
    assert!(began.began);
    assert!(watch.is_loading());
    for scene_state in [3, 4, 0, 0] {
        let step = watch.step(&readings(scene_state, 0x200, true));
        assert!(!step.began && !step.ended);
    }
    let ended = watch.step(&readings(1, 0x200, true));
    assert!(ended.ended && !ended.link_lost);
    assert!(!watch.is_loading());
    assert!(watch.step(&readings(2, 0x200, true)).began);
}

#[test]
fn a_scene_that_was_never_seen_running_does_not_begin_a_load() {
    let mut watch = LoadWatch::default();
    assert!(!watch.step(&readings(0, 0x100, true)).began);
    assert!(!watch.step(&readings(2, 0x100, true)).began);
    assert!(!watch.is_loading());
}

#[test]
fn a_load_with_the_link_down_is_not_watched_and_does_not_begin_late() {
    let mut watch = running_watch();
    assert!(!watch.step(&readings(2, 0x100, false)).began);
    assert!(!watch.step(&readings(0, 0x100, true)).began);
    assert!(!watch.is_loading());
    watch.step(&readings(1, 0x100, true));
    assert!(watch.step(&readings(2, 0x100, true)).began);
}

#[test]
fn a_link_that_drops_during_a_load_ends_it() {
    let mut watch = running_watch();
    watch.step(&readings(2, 0x100, true));
    let step = watch.step(&readings(0, 0x100, false));
    assert!(step.ended && step.link_lost);
    assert!(!watch.is_loading());
    assert!(!watch.step(&readings(0, 0x100, true)).began);
}

#[test]
fn a_system_phase_is_marked_while_the_load_runs() {
    let mut watch = running_watch();
    watch.step(&readings(2, 0x100, true));
    assert!(
        !watch
            .step(&readings(0, FIRST_SYSTEM_PHASE_ID - 1, true))
            .phase_is_a_system_phase
    );
    assert!(
        watch
            .step(&readings(0, FIRST_SYSTEM_PHASE_ID, true))
            .phase_is_a_system_phase
    );
}

#[test]
fn an_unreadable_scene_state_changes_nothing() {
    let mut watch = running_watch();
    let unreadable = LoadWatchReadings {
        scene_state: None,
        phase_id: None,
        campaign_link_up: true,
    };
    assert!(!watch.step(&unreadable).began);
    assert!(watch.step(&readings(2, 0x100, true)).began);
    assert!(!watch.step(&unreadable).ended);
    assert!(watch.is_loading());
}

#[test]
fn a_reset_forgets_a_load_in_progress() {
    let mut watch = running_watch();
    watch.step(&readings(2, 0x100, true));
    watch.reset();
    assert!(!watch.is_loading());
    assert!(!watch.step(&readings(0, 0x100, true)).began);
}
