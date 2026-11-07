use std::collections::{BTreeMap, BTreeSet};

use rc_game::enemies::{
    ENEMY_LIST_DIFF_INTERVAL_MS, EnemyLog, EnemyRoster, EnemySighting, ListDiff, ListDiffWatch,
    LocalKeep, RemovalCause,
};
use rc_proto::EnemyId;

const PHASE: u16 = 12;

fn sighting(actor: u64, group: u16, serial: i32, hp: i32) -> EnemySighting {
    EnemySighting {
        actor,
        spawn_serial: serial,
        group,
        class_id: 0x4b,
        position: [1.0, 2.0, 3.0],
        yaw_radians: 0.0,
        motion: None,
        hp,
        max_hp: 50,
    }
}

fn id(group: u16, ordinal: u16) -> EnemyId {
    EnemyId { group, ordinal }
}

fn host_spawn(_: EnemyId) -> String {
    "host spawn".to_owned()
}

#[test]
fn a_spawn_line_names_id_group_class_position_and_cause() {
    let mut roster = EnemyRoster::new();
    let mut log = EnemyLog::new();
    let update = roster.observe(PHASE, &[sighting(0xA, 19, 1, 50)]);
    let lines = log.observe(&update, host_spawn);
    assert_eq!(
        lines[0],
        "spawn 19:0 group 19 class 0x4b hp 50/50 at (1.0, 2.0, 3.0), cause: host spawn"
    );
}

#[test]
fn a_noted_cause_is_used_for_the_removal_line_once() {
    let mut roster = EnemyRoster::new();
    let mut log = EnemyLog::new();
    log.observe(
        &roster.observe(PHASE, &[sighting(0xA, 19, 1, 50)]),
        host_spawn,
    );
    log.note_cause(id(19, 0), RemovalCause::SilentRemoval);
    let lines = log.observe(&roster.observe(PHASE, &[]), host_spawn);
    assert!(
        lines[0].starts_with("removed 19:0 group 19 class 0x4b hp 50/50 last at (1.0, 2.0, 3.0)")
    );
    assert!(lines[0].ends_with(RemovalCause::SilentRemoval.text()));
}

#[test]
fn a_removal_without_a_noted_cause_is_told_apart_by_the_last_hp() {
    let mut roster = EnemyRoster::new();
    let mut log = EnemyLog::new();
    log.observe(
        &roster.observe(PHASE, &[sighting(0xA, 4, 1, 50), sighting(0xB, 5, 2, 0)]),
        host_spawn,
    );
    let lines = log.observe(&roster.observe(PHASE, &[]), host_spawn);
    let killed = lines
        .iter()
        .find(|line| line.contains("removed 5:0"))
        .expect("line");
    let removed = lines
        .iter()
        .find(|line| line.contains("removed 4:0"))
        .expect("line");
    assert!(killed.ends_with(RemovalCause::KilledLocally.text()));
    assert!(removed.ends_with(RemovalCause::GameRemoval.text()));
}

#[test]
fn leaving_the_zone_logs_each_enemy_as_a_zone_unload() {
    let mut roster = EnemyRoster::new();
    let mut log = EnemyLog::new();
    log.observe(
        &roster.observe(PHASE, &[sighting(0xA, 4, 1, 50), sighting(0xB, 4, 2, 50)]),
        host_spawn,
    );
    let lines = log.observe(&roster.observe(PHASE + 1, &[]), host_spawn);
    let unloads = lines
        .iter()
        .filter(|line| line.ends_with(RemovalCause::ZoneUnload.text()))
        .count();
    assert_eq!(unloads, 2);
}

#[test]
fn a_group_is_logged_when_it_registers_and_when_it_empties_not_per_kill() {
    let mut roster = EnemyRoster::new();
    let mut log = EnemyLog::new();
    let first = log.observe(
        &roster.observe(PHASE, &[sighting(0xA, 7, 1, 50), sighting(0xB, 7, 2, 50)]),
        host_spawn,
    );
    assert!(first.contains(&"group 7 registered with 2 enemies".to_owned()));
    let one_gone = log.observe(
        &roster.observe(PHASE, &[sighting(0xB, 7, 2, 50)]),
        host_spawn,
    );
    assert!(one_gone.iter().all(|line| !line.starts_with("group")));
    let empty = log.observe(&roster.observe(PHASE, &[]), host_spawn);
    assert!(empty.contains(&"group 7 is empty".to_owned()));
}

fn diff_only_host(ordinal: u16) -> ListDiff {
    ListDiff {
        only_host: BTreeSet::from([id(3, ordinal)]),
        only_local: BTreeMap::new(),
    }
}

#[test]
fn a_difference_is_logged_once_and_not_again_while_it_stays_the_same() {
    let mut watch = ListDiffWatch::new();
    let first = watch.observe(0, diff_only_host(0)).expect("logged");
    assert!(first.contains("only the host has [3:0 (no copy here)]"));
    assert_eq!(
        watch.observe(ENEMY_LIST_DIFF_INTERVAL_MS, diff_only_host(0)),
        None
    );
}

#[test]
fn the_lists_are_compared_only_every_five_seconds() {
    let mut watch = ListDiffWatch::new();
    watch.observe(0, ListDiff::default());
    assert_eq!(watch.observe(1000, diff_only_host(0)), None);
    assert!(
        watch
            .observe(ENEMY_LIST_DIFF_INTERVAL_MS, diff_only_host(0))
            .is_some()
    );
}

#[test]
fn a_changed_difference_is_logged_again_and_so_is_the_return_to_a_match() {
    let mut watch = ListDiffWatch::new();
    let step = ENEMY_LIST_DIFF_INTERVAL_MS;
    watch.observe(0, diff_only_host(0));
    assert!(watch.observe(step, diff_only_host(1)).is_some());
    let kept = ListDiff {
        only_host: BTreeSet::new(),
        only_local: BTreeMap::from([(id(23, 0), LocalKeep::HeldHidden)]),
    };
    let text = watch.observe(step * 2, kept).expect("logged");
    assert!(text.contains("only you have [23:0 (hidden and held, the host lacks it)]"));
    assert_eq!(
        watch.observe(step * 3, ListDiff::default()),
        Some("lists match again".to_owned())
    );
    assert_eq!(watch.observe(step * 4, ListDiff::default()), None);
}
