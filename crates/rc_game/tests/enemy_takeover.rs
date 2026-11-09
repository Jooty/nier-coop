use std::collections::HashSet;

use rc_game::enemies::{EnemyRoster, EnemySighting, plan_takeovers};
use rc_proto::EnemyId;

const PHASE: u16 = 3;
const GROUP: u16 = 6;
const WAVE_SIZE: u16 = 10;
const FIRST_COPY_ACTOR: u64 = 1000;
const FIRST_OWN_ACTOR: u64 = 2000;
const WOLF_CLASS: i32 = 40;

fn id(ordinal: u16) -> EnemyId {
    EnemyId {
        group: GROUP,
        ordinal,
    }
}

fn sighting(actor: u64, serial: i32, class_id: i32, x: f32) -> EnemySighting {
    EnemySighting {
        actor,
        spawn_serial: serial,
        group: GROUP,
        class_id,
        position: [x, 0.0, 0.0],
        yaw_radians: 0.0,
        motion: None,
        hp: 10,
        max_hp: 10,
    }
}

fn copy_sightings() -> Vec<EnemySighting> {
    (0..WAVE_SIZE)
        .map(|n| {
            sighting(
                FIRST_COPY_ACTOR + u64::from(n),
                i32::from(n),
                WOLF_CLASS,
                f32::from(n) * 5.0,
            )
        })
        .collect()
}

fn own_sightings() -> Vec<EnemySighting> {
    (0..WAVE_SIZE)
        .rev()
        .map(|n| {
            sighting(
                FIRST_OWN_ACTOR + u64::from(n),
                100 + i32::from(n),
                WOLF_CLASS,
                f32::from(n) * 5.0 + 0.2,
            )
        })
        .collect()
}

fn roster_with_copies() -> EnemyRoster {
    let mut roster = EnemyRoster::new();
    roster.observe(PHASE, &[]);
    for sighting in copy_sightings() {
        roster.claim(
            sighting.actor,
            sighting.spawn_serial,
            id(u16::try_from(sighting.actor - FIRST_COPY_ACTOR).unwrap()),
            sighting.hp,
        );
    }
    roster
}

#[test]
fn own_spawns_at_the_copies_positions_take_over_every_copy() {
    let mut roster = roster_with_copies();
    let mut all = copy_sightings();
    all.extend(own_sightings());
    let mut update = roster.observe(PHASE, &all);
    assert_eq!(update.spawned.len(), usize::from(WAVE_SIZE));
    let host_live: Vec<EnemyId> = (0..WAVE_SIZE).map(id).collect();
    let own_spawns: Vec<_> = update
        .seen
        .iter()
        .filter(|(id, _)| update.spawned.contains(id))
        .copied()
        .collect();
    let copies: Vec<_> = update
        .seen
        .iter()
        .filter(|(id, _)| host_live.contains(id))
        .copied()
        .collect();
    let plan = plan_takeovers(&own_spawns, &copies, &host_live, &HashSet::new());
    assert_eq!(plan.len(), usize::from(WAVE_SIZE));
    for (own, copy) in &plan {
        assert!(roster.take_over_copy(*own, *copy));
        let copy_actor = copies.iter().find(|(id, _)| id == copy).unwrap().1.actor;
        update
            .seen
            .retain(|(_, sighting)| sighting.actor != copy_actor);
    }
    update.rename(&plan);
    assert_eq!(update.seen.len(), usize::from(WAVE_SIZE));
    for (seen_id, sighting) in &update.seen {
        assert!(host_live.contains(seen_id));
        assert!(sighting.actor >= FIRST_OWN_ACTOR);
        assert_eq!(
            sighting.actor - FIRST_OWN_ACTOR,
            u64::from(seen_id.ordinal),
            "own actor at the same spot holds the host id"
        );
    }
    let next = roster.observe(PHASE, &own_sightings());
    assert!(next.spawned.is_empty());
    assert_eq!(next.seen.len(), usize::from(WAVE_SIZE));
    assert_eq!(next.gone.len(), usize::from(WAVE_SIZE));
}

#[test]
fn an_own_spawn_whose_id_the_host_has_is_left_alone() {
    let own = [(id(3), sighting(FIRST_OWN_ACTOR, 1, WOLF_CLASS, 0.0))];
    let copies = [(id(0), sighting(FIRST_COPY_ACTOR, 0, WOLF_CLASS, 0.0))];
    assert!(plan_takeovers(&own, &copies, &[id(0), id(3)], &HashSet::new()).is_empty());
}

#[test]
fn a_copy_of_another_kind_is_kept() {
    let own = [(id(10), sighting(FIRST_OWN_ACTOR, 1, WOLF_CLASS, 0.0))];
    let copies = [(id(0), sighting(FIRST_COPY_ACTOR, 0, WOLF_CLASS + 1, 0.0))];
    assert!(plan_takeovers(&own, &copies, &[id(0)], &HashSet::new()).is_empty());
}

#[test]
fn each_copy_is_taken_by_the_nearest_own_spawn_only_once() {
    let own = [
        (id(10), sighting(FIRST_OWN_ACTOR, 1, WOLF_CLASS, 1.0)),
        (id(11), sighting(FIRST_OWN_ACTOR + 1, 2, WOLF_CLASS, 0.5)),
    ];
    let copies = [(id(0), sighting(FIRST_COPY_ACTOR, 0, WOLF_CLASS, 0.0))];
    assert_eq!(
        plan_takeovers(&own, &copies, &[id(0)], &HashSet::new()),
        vec![(id(11), id(0))]
    );
}

#[test]
fn an_id_an_own_actor_already_holds_is_never_taken_over_again() {
    let own = [(id(11), sighting(FIRST_OWN_ACTOR + 1, 2, WOLF_CLASS, 0.0))];
    let copies = [(id(0), sighting(FIRST_OWN_ACTOR, 1, WOLF_CLASS, 0.0))];
    let held = HashSet::from([id(0)]);
    assert!(plan_takeovers(&own, &copies, &[id(0)], &held).is_empty());
}

#[test]
fn a_far_spawn_does_not_pair_with_a_copy() {
    let own = [(id(10), sighting(FIRST_OWN_ACTOR, 1, WOLF_CLASS, 30.0))];
    let copies = [(id(0), sighting(FIRST_COPY_ACTOR, 0, WOLF_CLASS, 0.0))];
    assert!(plan_takeovers(&own, &copies, &[id(0)], &HashSet::new()).is_empty());
}
