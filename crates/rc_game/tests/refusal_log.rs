use std::time::{Duration, Instant};

use rc_game::link_traffic::{REFUSAL_LOG_INTERVAL, RefusalLog};

#[test]
fn the_first_refusal_of_a_kind_is_logged() {
    let mut log = RefusalLog::default();
    assert!(log.should_log("SceneFollow", Instant::now()));
}

#[test]
fn a_kind_is_logged_once_per_interval() {
    let start = Instant::now();
    let mut log = RefusalLog::default();
    assert!(log.should_log("SceneFollow", start));
    assert!(!log.should_log("SceneFollow", start + Duration::from_secs(1)));
    assert!(!log.should_log(
        "SceneFollow",
        start + REFUSAL_LOG_INTERVAL - Duration::from_millis(1)
    ));
    assert!(log.should_log("SceneFollow", start + REFUSAL_LOG_INTERVAL));
}

#[test]
fn another_kind_is_not_silenced_by_the_first() {
    let start = Instant::now();
    let mut log = RefusalLog::default();
    assert!(log.should_log("SceneFollow", start));
    assert!(log.should_log("CallbackFired", start));
}
