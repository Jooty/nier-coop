use std::time::{Duration, Instant};

use replicant_game::link_traffic::LinkTraffic;

#[test]
fn a_fresh_link_has_seen_nothing() {
    let traffic = LinkTraffic::default();
    assert_eq!(
        traffic.describe(Instant::now()),
        "last sent nothing; last received nothing"
    );
}

#[test]
fn the_latest_message_of_each_direction_is_named_with_its_age() {
    let start = Instant::now();
    let mut traffic = LinkTraffic::default();
    traffic.note_sent("State", start);
    traffic.note_sent("DoorOpened", start + Duration::from_millis(100));
    traffic.note_received("Ping", start + Duration::from_millis(50));
    assert_eq!(
        traffic.describe(start + Duration::from_millis(600)),
        "last sent DoorOpened 500 ms ago; last received Ping 550 ms ago"
    );
}

#[test]
fn forgetting_clears_both_directions() {
    let now = Instant::now();
    let mut traffic = LinkTraffic::default();
    traffic.note_sent("State", now);
    traffic.forget();
    assert_eq!(
        traffic.describe(now),
        "last sent nothing; last received nothing"
    );
}
