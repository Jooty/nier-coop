use std::collections::HashSet;

use rc_game::enemies::{EnemyRoster, EnemySighting, plan_fight_bindings, plan_takeovers};
use rc_proto::EnemyId;

const PHASE: u16 = 3;
const GROUP: u16 = 6;
const WAVE_SIZE: u16 = 10;
const FIRST_COPY_ACTOR: u64 = 1000;
const FIRST_OWN_ACTOR: u64 = 2000;
const WOLF_CLASS: i32 = 40;
const RUN34_WAVE_CLASS: i32 = 0x15;
const RUN34_WAVES_BEFORE: u16 = 3;
const RUN34_HOST_WAVE_30: [(f32, f32); 10] = [
    (-1.1, -11.7),
    (-6.7, -17.3),
    (2.6, -21.0),
    (4.4, -13.6),
    (6.3, -15.4),
    (4.4, -22.9),
    (-1.1, -17.3),
    (6.3, -21.0),
    (2.6, -17.3),
    (4.4, -17.3),
];
const RUN34_HOST_WAVE_40: [(f32, f32); 10] = [
    (2.6, -13.6),
    (0.7, -17.3),
    (-3.0, -19.2),
    (2.6, -21.0),
    (-3.0, -21.0),
    (-4.9, -17.3),
    (-6.7, -15.4),
    (0.7, -15.4),
    (-3.0, -13.6),
    (0.7, -13.6),
];

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

fn wave_sightings(
    first_actor: u64,
    first_serial: i32,
    positions: &[(f32, f32)],
) -> Vec<EnemySighting> {
    positions
        .iter()
        .zip(0_u16..)
        .map(|(&(x, z), n)| EnemySighting {
            position: [x, 146.8, z],
            ..sighting(
                first_actor + u64::from(n),
                first_serial + i32::from(n),
                RUN34_WAVE_CLASS,
                0.0,
            )
        })
        .collect()
}

fn roster_after_the_first_waves() -> EnemyRoster {
    let mut roster = EnemyRoster::new();
    roster.observe(PHASE, &[]);
    for wave in 0..RUN34_WAVES_BEFORE {
        let first = u64::from(wave) * 100;
        let serial = i32::from(wave) * 100;
        roster.observe(PHASE, &wave_sightings(first, serial, &RUN34_HOST_WAVE_40));
        roster.observe(PHASE, &[]);
    }
    roster
}

fn bind_inside_the_window(
    roster: &mut EnemyRoster,
    spawned: &[EnemyId],
    taken_over: &HashSet<EnemyId>,
) -> Vec<(EnemyId, EnemyId)> {
    let own: Vec<(EnemyId, u16)> = spawned
        .iter()
        .filter_map(|id| roster.own_arrival_of(*id).map(|arrival| (*id, arrival)))
        .collect();
    let plan = plan_fight_bindings(&own, &roster.held_ids(), taken_over);
    roster.bind_ids(&plan)
}

fn host_wave_position(ordinal: u16) -> [f32; 3] {
    let wave = if ordinal < 40 {
        RUN34_HOST_WAVE_30
    } else {
        RUN34_HOST_WAVE_40
    };
    let (x, z) = wave[usize::from(ordinal % 10)];
    [x, 146.8, z]
}

#[test]
fn run34_plan_c_copies_made_the_guests_own_late_wave_take_the_next_waves_ids() {
    // run34 G 5902-5911: the guest sat in a menu, so after 3 s the host's 6:30..39 were made as
    // plan C copies; its own wave of those enemies came 22 s later and was numbered 6:40..49.
    let mut roster = roster_after_the_first_waves();
    for (n, _) in RUN34_HOST_WAVE_30.iter().enumerate() {
        let ordinal = 30 + u16::try_from(n).unwrap();
        roster.claim(5_000 + u64::from(ordinal), 5_000, id(ordinal), 24);
    }
    let own_wave = wave_sightings(9_000, 9_000, &RUN34_HOST_WAVE_30);
    let update = roster.observe(PHASE, &own_wave);
    assert_eq!(update.spawned, (40..50).map(id).collect::<Vec<_>>());
    for (seen_id, sighting) in &update.seen {
        assert_ne!(
            sighting.position,
            host_wave_position(seen_id.ordinal),
            "own actor {seen_id:?} stands where the host's wave 30 enemy is, not its own id's"
        );
    }
}

#[test]
fn run34_doubled_waves_inside_a_fight_window_give_one_enemy_per_host_id() {
    // Inside the window no plan C copy is made: the guest's own replayed wave numbers itself.
    let mut roster = roster_after_the_first_waves();
    let taken_over = HashSet::new();
    let wave_30 = wave_sightings(9_000, 9_000, &RUN34_HOST_WAVE_30);
    let update = roster.observe(PHASE, &wave_30);
    assert!(bind_inside_the_window(&mut roster, &update.spawned, &taken_over).is_empty());
    assert_eq!(update.spawned, (30..40).map(id).collect::<Vec<_>>());
    let mut both = wave_30;
    both.extend(wave_sightings(9_100, 9_100, &RUN34_HOST_WAVE_40));
    let update = roster.observe(PHASE, &both);
    assert!(bind_inside_the_window(&mut roster, &update.spawned, &taken_over).is_empty());
    let held: Vec<EnemyId> = update.seen.iter().map(|(seen_id, _)| *seen_id).collect();
    assert_eq!(
        held,
        (30..50).map(id).collect::<Vec<_>>(),
        "one actor per host id"
    );
    for (seen_id, sighting) in &update.seen {
        assert_eq!(sighting.position, host_wave_position(seen_id.ordinal));
    }
}

#[test]
fn copies_made_before_the_window_no_longer_shift_the_replayed_waves_ids() {
    let mut roster = roster_after_the_first_waves();
    for ordinal in 30..40 {
        roster.claim(5_000 + u64::from(ordinal), 5_000, id(ordinal), 24);
    }
    roster.observe(PHASE, &[]);
    let wave_30 = wave_sightings(9_000, 9_000, &RUN34_HOST_WAVE_30);
    let mut update = roster.observe(PHASE, &wave_30);
    assert_eq!(update.spawned, (40..50).map(id).collect::<Vec<_>>());
    let made = bind_inside_the_window(&mut roster, &update.spawned, &HashSet::new());
    assert_eq!(made.len(), 10);
    update.rename(&made);
    for (seen_id, sighting) in &update.seen {
        assert!((30..40).contains(&seen_id.ordinal));
        assert_eq!(sighting.position, host_wave_position(seen_id.ordinal));
    }
    let next = roster.observe(PHASE, &wave_30);
    assert!(next.spawned.is_empty());
    assert_eq!(next.seen.len(), 10);
}

#[test]
fn a_fight_spawn_never_binds_to_an_id_held_here_or_ever_taken_over() {
    let own = [(id(40), 30), (id(41), 31), (id(42), 32)];
    let held: HashSet<EnemyId> = [id(30), id(40), id(41), id(42)].into_iter().collect();
    let taken_over: HashSet<EnemyId> = [id(31)].into_iter().collect();
    assert_eq!(
        plan_fight_bindings(&own, &held, &taken_over),
        vec![(id(42), id(32))]
    );
}
