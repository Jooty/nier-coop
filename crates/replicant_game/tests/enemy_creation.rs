use std::collections::HashSet;

use replicant_game::enemies::{
    CreationWatch, EnemyRoster, EnemySighting, GUEST_ONLY_ENEMY_GRACE_MS, GuestOnlyWatch,
    HOST_SPAWN_COPY_GRACE_MS, creation_matrix_scale, missing_host_spawns, scaled_matrix,
    spec_slot_address, spec_slot_index,
};
use replicant_game::offsets::{ENEMY_SPEC_SLOT_BYTES, ENEMY_SPEC_SLOT_COUNT};
use replicant_proto::{ENEMY_SPEC_BYTES, EnemyId};

const MODULE_BASE: u64 = 0x1_4000_0000;
const PHASE: u16 = 3;

fn id(group: u16, ordinal: u16) -> EnemyId {
    EnemyId { group, ordinal }
}

fn sighting(actor: u64, group: u16, serial: i32) -> EnemySighting {
    EnemySighting {
        actor,
        spawn_serial: serial,
        group,
        class_id: 0,
        position: [0.0; 3],
        yaw_radians: 0.0,
        motion: None,
        hp: 10,
        max_hp: 10,
    }
}

#[test]
fn the_spec_size_on_the_wire_is_the_games_slot_size() {
    assert_eq!(ENEMY_SPEC_BYTES, ENEMY_SPEC_SLOT_BYTES);
}

#[test]
fn a_missing_host_spawn_gets_a_copy_only_after_the_grace() {
    let mut watch = CreationWatch::new();
    let missing = [id(100, 1)];
    assert!(watch.due(1000, &missing).is_empty());
    assert!(
        watch
            .due(1000 + HOST_SPAWN_COPY_GRACE_MS - 1, &missing)
            .is_empty()
    );
    assert_eq!(
        watch.due(1000 + HOST_SPAWN_COPY_GRACE_MS, &missing),
        vec![id(100, 1)]
    );
    assert!(
        watch
            .due(1000 + HOST_SPAWN_COPY_GRACE_MS + 1, &missing)
            .is_empty()
    );
}

#[test]
fn an_own_spawn_inside_the_grace_means_no_copy_is_made() {
    let mut watch = CreationWatch::new();
    assert!(watch.due(0, &[id(100, 1)]).is_empty());
    assert!(watch.due(2000, &[]).is_empty());
    assert!(watch.due(2500, &[id(100, 1)]).is_empty());
    assert!(
        watch
            .due(2500 + HOST_SPAWN_COPY_GRACE_MS - 1, &[id(100, 1)])
            .is_empty()
    );
}

#[test]
fn only_live_host_spawns_with_a_known_creation_that_are_not_here_are_missing() {
    let host_live = [id(100, 1), id(100, 2), id(100, 3), id(100, 4), id(101, 0)];
    let own: HashSet<EnemyId> = [id(100, 1)].into();
    let created: HashSet<EnemyId> = [id(100, 2)].into();
    let ended: HashSet<EnemyId> = [id(100, 3)].into();
    let known: HashSet<EnemyId> = [id(100, 1), id(100, 2), id(100, 3), id(100, 4)].into();
    assert_eq!(
        missing_host_spawns(&host_live, &own, &created, &ended, &known),
        vec![id(100, 4)]
    );
}

#[test]
fn a_created_copy_keeps_the_hosts_id_and_the_next_own_spawn_does_not_collide() {
    let mut roster = EnemyRoster::new();
    roster.observe(PHASE, &[]);
    roster.claim(0x500, 40, id(100, 1), 10);
    let update = roster.observe(PHASE, &[sighting(0x500, 100, 40)]);
    assert!(update.spawned.is_empty());
    assert_eq!(update.seen.len(), 1);
    assert_eq!(update.seen[0].0, id(100, 1));
    let update = roster.observe(PHASE, &[sighting(0x500, 100, 40), sighting(0x600, 100, 41)]);
    assert_eq!(update.spawned, vec![id(100, 2)]);
}

#[test]
fn an_own_spawn_that_duplicates_a_created_copy_is_hidden_after_the_grace_not_kept_as_a_second_enemy()
 {
    let mut roster = EnemyRoster::new();
    roster.observe(PHASE, &[]);
    roster.claim(0x500, 40, id(100, 1), 10);
    let update = roster.observe(PHASE, &[sighting(0x500, 100, 40), sighting(0x600, 100, 41)]);
    let host_live = [id(100, 1)];
    let adoptions = roster.adopt_host_ids(&update.spawned, &host_live);
    assert!(adoptions.is_empty());
    let guest_only: Vec<u64> = update
        .seen
        .iter()
        .filter(|(seen, _)| !host_live.contains(seen))
        .map(|(_, sighting)| sighting.actor)
        .collect();
    assert_eq!(guest_only, vec![0x600]);
    let mut watch = GuestOnlyWatch::new();
    assert!(watch.expired(0, &guest_only).is_empty());
    assert_eq!(
        watch.expired(GUEST_ONLY_ENEMY_GRACE_MS, &guest_only),
        vec![0x600]
    );
}

#[test]
fn spec_slots_map_to_addresses_and_back() {
    for slot in [0_u16, 1, 77, 255] {
        let address = spec_slot_address(MODULE_BASE, slot).expect("in range");
        assert_eq!(spec_slot_index(MODULE_BASE, address), Some(slot));
    }
    let first = spec_slot_address(MODULE_BASE, 0).expect("in range");
    assert_eq!(spec_slot_address(MODULE_BASE, 256), None);
    assert_eq!(spec_slot_index(MODULE_BASE, first - 1), None);
    assert_eq!(spec_slot_index(MODULE_BASE, first + 1), None);
    assert_eq!(
        spec_slot_index(
            MODULE_BASE,
            first + (ENEMY_SPEC_SLOT_COUNT * ENEMY_SPEC_SLOT_BYTES) as u64
        ),
        None
    );
}

#[test]
fn the_creation_matrix_carries_position_and_scale() {
    let matrix = scaled_matrix([1.0, 2.0, 3.0], 0.0, 2.0);
    assert!((creation_matrix_scale(&matrix) - 2.0).abs() < 1e-6);
    assert_eq!(&matrix[12..16], &1.0_f32.to_le_bytes());
    assert_eq!(&matrix[28..32], &2.0_f32.to_le_bytes());
    assert_eq!(&matrix[44..48], &3.0_f32.to_le_bytes());
    let fallback = scaled_matrix([0.0; 3], 0.0, f32::NAN);
    assert!((creation_matrix_scale(&fallback) - 1.0).abs() < 1e-6);
}
