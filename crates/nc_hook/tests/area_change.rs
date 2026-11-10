use nc_hook::menu::{
    AREA_CHANGE_TIMEOUT_MS, AreaChangeOutcome, AreaChangeRefusal, AreaChangeTracker, AreaEntry,
};

fn area(id: u32, name: &str) -> AreaEntry {
    AreaEntry {
        id,
        name: name.to_owned(),
    }
}

#[test]
fn the_area_is_reported_changed_only_once_it_really_changed() {
    let mut tracker = AreaChangeTracker::default();
    let meadow = area(1, "Meadow");
    let cave = area(2, "Cave");
    tracker
        .request(&cave, Some(&meadow), None, 1_000)
        .expect("allowed");
    assert!(tracker.is_waiting());
    assert_eq!(tracker.message(), "changing to Cave");
    assert_eq!(tracker.observe(Some(&meadow), 2_000), None);
    assert_eq!(
        tracker.observe(Some(&cave), 3_000),
        Some(AreaChangeOutcome::Changed("Cave".to_owned()))
    );
    assert_eq!(tracker.message(), "changed to Cave");
    assert!(!tracker.is_waiting());
}

#[test]
fn a_guest_is_refused_with_the_reason() {
    let mut tracker = AreaChangeTracker::default();
    let refusal = tracker
        .request(
            &area(2, "Cave"),
            Some(&area(1, "Meadow")),
            Some(AreaChangeRefusal::NotHost),
            0,
        )
        .expect_err("refused");
    assert_eq!(refusal, AreaChangeRefusal::NotHost);
    assert!(!tracker.is_waiting());
    assert_eq!(
        tracker.message(),
        "not changed: only the host may change the area"
    );
}

#[test]
fn a_scene_a_talk_and_a_pop_up_each_name_themselves() {
    let cases = [
        (
            AreaChangeRefusal::SceneRunning,
            "refused while a scene runs",
        ),
        (AreaChangeRefusal::TalkRunning, "refused while a talk runs"),
        (
            AreaChangeRefusal::PopUpRunning,
            "refused while a pop-up runs",
        ),
    ];
    for (refusal, text) in cases {
        let mut tracker = AreaChangeTracker::default();
        let result = tracker.request(&area(2, "Cave"), None, Some(refusal), 0);
        assert!(result.is_err());
        assert_eq!(tracker.message(), format!("not changed: {text}"));
    }
}

#[test]
fn asking_for_the_area_you_are_in_is_refused() {
    let mut tracker = AreaChangeTracker::default();
    let meadow = area(1, "Meadow");
    let refusal = tracker
        .request(&meadow, Some(&meadow), None, 0)
        .expect_err("already there");
    assert_eq!(refusal, AreaChangeRefusal::AlreadyThere);
}

#[test]
fn no_change_after_fifteen_seconds_is_reported_once() {
    let mut tracker = AreaChangeTracker::default();
    let meadow = area(1, "Meadow");
    tracker
        .request(&area(2, "Cave"), Some(&meadow), None, 5_000)
        .expect("allowed");
    assert_eq!(
        tracker.observe(Some(&meadow), 5_000 + AREA_CHANGE_TIMEOUT_MS - 1),
        None
    );
    assert_eq!(
        tracker.observe(Some(&meadow), 5_000 + AREA_CHANGE_TIMEOUT_MS),
        Some(AreaChangeOutcome::TimedOut("Cave".to_owned()))
    );
    assert_eq!(tracker.observe(Some(&meadow), 99_999), None);
    assert_eq!(
        tracker.message(),
        "not changed to Cave: no change after 15 s"
    );
}

#[test]
fn the_timeout_is_fifteen_seconds() {
    assert_eq!(AREA_CHANGE_TIMEOUT_MS, 15_000);
}

#[test]
fn the_game_refusing_the_call_is_shown_as_the_reason() {
    let mut tracker = AreaChangeTracker::default();
    tracker
        .request(&area(2, "Cave"), None, None, 0)
        .expect("allowed");
    tracker.game_refused("the queue is full");
    assert!(!tracker.is_waiting());
    assert_eq!(tracker.message(), "not changed: the queue is full");
}

#[test]
fn outcomes_have_teleport_log_lines() {
    assert_eq!(
        AreaChangeOutcome::Changed("Cave".to_owned()).log_line(),
        "teleport: area changed to Cave"
    );
    assert_eq!(
        AreaChangeOutcome::TimedOut("Cave".to_owned()).log_line(),
        "teleport: no change to Cave after 15 s"
    );
}

#[test]
fn a_new_request_replaces_a_waiting_one() {
    let mut tracker = AreaChangeTracker::default();
    let meadow = area(1, "Meadow");
    tracker
        .request(&area(2, "Cave"), Some(&meadow), None, 0)
        .expect("allowed");
    tracker
        .request(&area(3, "Lake"), Some(&meadow), None, 1_000)
        .expect("allowed");
    assert_eq!(
        tracker.observe(Some(&area(3, "Lake")), 2_000),
        Some(AreaChangeOutcome::Changed("Lake".to_owned()))
    );
}
