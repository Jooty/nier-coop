use replicant_game::desync::{
    DESYNC_DIGEST_INTERVAL_MS, DesyncReporter, ids_hash, live_group_rosters,
};
use replicant_proto::{DesyncDigest, MAX_DESYNC_GROUPS};

fn digest(phase: u16) -> DesyncDigest {
    DesyncDigest {
        phase,
        position: [0.0; 3],
        story: None,
        groups: Vec::new(),
        groups_not_listed: 0,
    }
}

#[test]
fn the_ids_hash_ignores_order_and_tells_sets_apart() {
    assert_eq!(ids_hash(&[3, 1, 2]), ids_hash(&[1, 2, 3]));
    assert_ne!(ids_hash(&[1, 2, 3]), ids_hash(&[1, 2, 4]));
}

#[test]
fn live_actors_become_one_roster_per_group_in_group_order() {
    let (rosters, not_listed) = live_group_rosters(&[(9, 5), (2, 7), (9, 6)]);
    assert_eq!(not_listed, 0);
    assert_eq!(rosters.len(), 2);
    assert_eq!((rosters[0].group, rosters[0].count), (2, 1));
    assert_eq!((rosters[1].group, rosters[1].count), (9, 2));
    assert_eq!(rosters[1].ids_hash, ids_hash(&[5, 6]));
}

#[test]
fn groups_past_the_limit_are_counted_not_listed() {
    let live: Vec<(u16, i32)> = (0..MAX_DESYNC_GROUPS as u16 + 3)
        .map(|group| (group, 1))
        .collect();
    let (rosters, not_listed) = live_group_rosters(&live);
    assert_eq!(rosters.len(), MAX_DESYNC_GROUPS);
    assert_eq!(not_listed, 3);
}

#[test]
fn a_digest_is_due_every_five_seconds() {
    let mut reporter = DesyncReporter::default();
    assert!(reporter.digest_is_due(1000));
    assert!(!reporter.digest_is_due(1000 + DESYNC_DIGEST_INTERVAL_MS - 1));
    assert!(reporter.digest_is_due(1000 + DESYNC_DIGEST_INTERVAL_MS));
}

#[test]
fn a_mismatch_is_logged_every_time_with_the_desync_prefix() {
    let mut reporter = DesyncReporter::default();
    for now in [0, 5000] {
        assert_eq!(
            reporter.compare(now, &digest(1), &digest(2), None),
            vec!["desync: phase (yours 1, partner 2)".to_owned()]
        );
    }
}

#[test]
fn in_step_is_logged_once_a_minute() {
    let mut reporter = DesyncReporter::default();
    assert_eq!(
        reporter.compare(0, &digest(1), &digest(1), None),
        vec!["desync: in step"]
    );
    assert!(
        reporter
            .compare(5000, &digest(1), &digest(1), None)
            .is_empty()
    );
    assert!(
        reporter
            .compare(59_999, &digest(1), &digest(1), None)
            .is_empty()
    );
    assert_eq!(
        reporter.compare(60_000, &digest(1), &digest(1), None).len(),
        1
    );
}
