use rc_game::enemies::{
    ENEMY_MIRROR_STALE_MS, EnemyMirror, EnemyRoster, EnemySighting, enemy_state, match_enemies,
    scan_enemies, shadow_matrix, split_snapshot,
};
use rc_game::offsets::{
    ACTOR_KIND_MASK_OFFSET, ACTOR_MANAGER_GROUP_COUNT_OFFSET, ACTOR_MANAGER_GROUP_STRIDE,
    ACTOR_MANAGER_RVA, ACTOR_SELF_HANDLE_OFFSET, ACTOR_SPAWN_SERIAL_OFFSET,
    ACTOR_TRANSFORM_BLOCK_BYTES, ACTOR_TRANSFORM_BLOCK_OFFSET, HANDLE_POOL_GENERATIONS_RVA,
    HANDLE_POOL_POINTERS_RVA, PUPPET_ACTOR_GROUP,
};
use rc_game::puppet::PuppetPose;
use rc_game::{GameMemory, Position};
use rc_proto::{
    ENEMY_GROUP_COUNT, EnemyGone, EnemyGoneReason, EnemyId, EnemySnapshot, EnemySpawned,
    EnemyState, MAX_ENEMIES_PER_SNAPSHOT, MAX_ENEMY_SNAPSHOT_PARTS, Message, encode,
};

use crate::fake_memory::FakeMemory;

const PHASE: u16 = 12;

fn sighting(actor: u64, group: u16, serial: i32) -> EnemySighting {
    EnemySighting {
        actor,
        spawn_serial: serial,
        group,
        position: [actor as f32, 0.0, 0.0],
        yaw_radians: 0.5,
        motion: None,
        hp: 50,
        max_hp: 50,
    }
}

fn id(group: u16, ordinal: u16) -> EnemyId {
    EnemyId { group, ordinal }
}

fn state(group: u16, ordinal: u16, x: f32) -> EnemyState {
    EnemyState {
        id: id(group, ordinal),
        position: [x, 0.0, 0.0],
        yaw_radians: 0.0,
        motion: None,
        hp: 10,
        max_hp: 20,
    }
}

fn snapshot(tick: u32, enemies: Vec<EnemyState>) -> EnemySnapshot {
    EnemySnapshot {
        tick,
        phase: PHASE,
        part: 0,
        parts: 1,
        total: u16::try_from(enemies.len()).expect("small"),
        enemies,
    }
}

#[test]
fn ordinals_follow_spawn_order_within_a_group_not_scan_order() {
    let mut roster = EnemyRoster::new();
    let update = roster.observe(
        PHASE,
        &[
            sighting(0xC000, 7, 30),
            sighting(0xA000, 7, 10),
            sighting(0xB000, 7, 20),
            sighting(0xD000, 9, 5),
        ],
    );
    let ids: Vec<(EnemyId, u64)> = update
        .seen
        .iter()
        .map(|(id, sighting)| (*id, sighting.actor))
        .collect();
    assert_eq!(
        ids,
        vec![
            (id(7, 0), 0xA000),
            (id(7, 1), 0xB000),
            (id(7, 2), 0xC000),
            (id(9, 0), 0xD000),
        ]
    );
    assert_eq!(update.spawned.len(), 4);
}

#[test]
fn two_machines_with_different_addresses_and_serials_agree_on_the_ids() {
    let mut host = EnemyRoster::new();
    let mut guest = EnemyRoster::new();
    let host_ids: Vec<EnemyId> = host
        .observe(PHASE, &[sighting(0x100, 3, 900), sighting(0x200, 3, 901)])
        .spawned;
    let guest_ids: Vec<EnemyId> = guest
        .observe(PHASE, &[sighting(0x7700, 3, 12), sighting(0x6600, 3, 14)])
        .spawned;
    assert_eq!(host_ids, guest_ids);
}

#[test]
fn an_enemy_keeps_its_id_while_others_come_and_go() {
    let mut roster = EnemyRoster::new();
    roster.observe(PHASE, &[sighting(0xA, 4, 1), sighting(0xB, 4, 2)]);
    let update = roster.observe(PHASE, &[sighting(0xB, 4, 2), sighting(0xC, 4, 3)]);
    assert_eq!(update.spawned, vec![id(4, 2)]);
    assert_eq!(update.gone, vec![(id(4, 0), EnemyGoneReason::Removed)]);
    let seen: Vec<EnemyId> = update.seen.iter().map(|(id, _)| *id).collect();
    assert_eq!(seen, vec![id(4, 1), id(4, 2)]);
}

#[test]
fn an_enemy_last_seen_at_zero_hp_is_reported_killed() {
    let mut roster = EnemyRoster::new();
    roster.observe(PHASE, &[sighting(0xA, 4, 1)]);
    let dying = EnemySighting {
        hp: 0,
        ..sighting(0xA, 4, 1)
    };
    roster.observe(PHASE, &[dying]);
    let update = roster.observe(PHASE, &[]);
    assert_eq!(update.gone, vec![(id(4, 0), EnemyGoneReason::Killed)]);
}

#[test]
fn a_reused_address_with_a_new_serial_is_a_new_enemy() {
    let mut roster = EnemyRoster::new();
    roster.observe(PHASE, &[sighting(0xA, 4, 1)]);
    let update = roster.observe(PHASE, &[sighting(0xA, 4, 2)]);
    assert_eq!(update.spawned, vec![id(4, 1)]);
    assert_eq!(update.gone, vec![(id(4, 0), EnemyGoneReason::Removed)]);
}

#[test]
fn a_new_phase_numbers_from_zero_again_without_death_events() {
    let mut roster = EnemyRoster::new();
    roster.observe(PHASE, &[sighting(0xA, 4, 1), sighting(0xB, 4, 2)]);
    let update = roster.observe(PHASE + 1, &[sighting(0xC, 4, 3)]);
    assert!(update.phase_changed);
    assert_eq!(update.spawned, vec![id(4, 0)]);
    assert!(update.gone.is_empty());
}

#[test]
fn a_sighting_becomes_a_valid_wire_state() {
    let odd = EnemySighting {
        yaw_radians: 7.0,
        hp: 80,
        max_hp: 60,
        ..sighting(0xA, 4, 1)
    };
    let wire = enemy_state(id(4, 0), &odd).expect("finite position");
    assert!(wire.yaw_radians.abs() <= std::f32::consts::PI);
    assert_eq!((wire.hp, wire.max_hp), (60, 60));
    let spawned = Message::EnemySpawned(EnemySpawned {
        phase: PHASE,
        enemy: wire,
    });
    assert!(encode(&spawned).is_ok());
    let lost = EnemySighting {
        position: [f32::NAN, 0.0, 0.0],
        ..sighting(0xA, 4, 1)
    };
    assert_eq!(enemy_state(id(4, 0), &lost), None);
}

#[test]
fn a_big_snapshot_is_split_into_parts_that_each_encode() {
    let enemies: Vec<EnemyState> = (0..12).map(|n| state(1, n, f32::from(n))).collect();
    let parts = split_snapshot(5, PHASE, enemies, &[[0.0; 3]]);
    assert_eq!(parts.len(), 3);
    for (index, part) in parts.iter().enumerate() {
        assert_eq!(usize::from(part.part), index);
        assert_eq!(part.parts, 3);
        assert_eq!(part.total, 12);
        assert!(encode(&Message::EnemySnapshot(part.clone())).is_ok());
    }
}

#[test]
fn when_too_many_enemies_the_nearest_to_either_player_are_sent() {
    let limit = MAX_ENEMIES_PER_SNAPSHOT * usize::from(MAX_ENEMY_SNAPSHOT_PARTS);
    let count = u16::try_from(limit + 3).expect("small");
    let enemies: Vec<EnemyState> = (0..count).map(|n| state(1, n, f32::from(n))).collect();
    let far_player = [f32::from(count - 1), 0.0, 0.0];
    let parts = split_snapshot(5, PHASE, enemies, &[[-1000.0, 0.0, 0.0], far_player]);
    let sent: Vec<u16> = parts
        .iter()
        .flat_map(|part| part.enemies.iter().map(|enemy| enemy.id.ordinal))
        .collect();
    assert_eq!(sent.len(), limit);
    assert_eq!(sent[0], count - 1);
    assert!(!sent.contains(&0));
    assert_eq!(parts[0].total, count);
}

#[test]
fn no_enemies_still_sends_one_empty_part() {
    let parts = split_snapshot(5, PHASE, Vec::new(), &[]);
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].total, 0);
}

#[test]
fn the_mirror_interpolates_between_snapshots() {
    let mut mirror = EnemyMirror::new();
    mirror.receive_snapshot(0, &snapshot(1, vec![state(2, 0, 0.0)]));
    mirror.receive_snapshot(100, &snapshot(2, vec![state(2, 0, 10.0)]));
    let sampled = mirror.sample(150, 100);
    assert_eq!(sampled.len(), 1);
    assert!((sampled[0].position[0] - 5.0).abs() < 1e-4);
    assert_eq!(mirror.host_total(), Some(1));
}

#[test]
fn a_dead_enemy_is_not_brought_back_by_a_late_snapshot() {
    let mut mirror = EnemyMirror::new();
    mirror.receive_snapshot(0, &snapshot(1, vec![state(2, 0, 0.0), state(2, 1, 0.0)]));
    mirror.receive_gone(&EnemyGone {
        phase: PHASE,
        id: id(2, 0),
        reason: EnemyGoneReason::Killed,
    });
    mirror.receive_snapshot(10, &snapshot(1, vec![state(2, 0, 0.0), state(2, 1, 0.0)]));
    let ids: Vec<EnemyId> = mirror.sample(10, 0).iter().map(|enemy| enemy.id).collect();
    assert_eq!(ids, vec![id(2, 1)]);
}

#[test]
fn an_enemy_the_host_stops_sending_is_dropped() {
    let mut mirror = EnemyMirror::new();
    mirror.receive_snapshot(0, &snapshot(1, vec![state(2, 0, 0.0)]));
    mirror.prune(ENEMY_MIRROR_STALE_MS);
    assert_eq!(mirror.len(), 1);
    mirror.prune(ENEMY_MIRROR_STALE_MS + 1);
    assert!(mirror.is_empty());
}

#[test]
fn a_message_from_a_new_phase_clears_the_mirror() {
    let mut mirror = EnemyMirror::new();
    mirror.receive_snapshot(0, &snapshot(1, vec![state(2, 0, 0.0)]));
    mirror.receive_spawned(
        5,
        &EnemySpawned {
            phase: PHASE + 1,
            enemy: state(8, 0, 1.0),
        },
    );
    assert_eq!(mirror.phase(), Some(PHASE + 1));
    let ids: Vec<EnemyId> = mirror.sample(5, 0).iter().map(|enemy| enemy.id).collect();
    assert_eq!(ids, vec![id(8, 0)]);
}

#[test]
fn matching_reports_pairs_strays_and_the_worst_error() {
    let local = [
        (id(1, 0), [0.0, 0.0, 0.0]),
        (id(1, 1), [0.0, 0.0, 0.0]),
        (id(3, 0), [0.0, 0.0, 0.0]),
    ];
    let remote = [
        (id(1, 0), [3.0, 4.0, 0.0]),
        (id(1, 1), [1.0, 0.0, 0.0]),
        (id(5, 0), [0.0, 0.0, 0.0]),
    ];
    let report = match_enemies(&local, &remote);
    assert_eq!((report.local_count, report.remote_count), (3, 3));
    assert_eq!(report.matched.len(), 2);
    assert_eq!(report.unmatched_local, vec![id(3, 0)]);
    assert_eq!(report.unmatched_remote, vec![id(5, 0)]);
    assert_eq!(report.worst_error(), Some((id(1, 0), 5.0)));
}

#[test]
fn the_shadow_matrix_keeps_the_enemys_scale_and_takes_the_hosts_pose() {
    let big = PuppetPose {
        position: Position {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        yaw: 0.3,
    }
    .creation_matrix();
    let mut block = [0_u8; ACTOR_TRANSFORM_BLOCK_BYTES];
    block.copy_from_slice(&big[..ACTOR_TRANSFORM_BLOCK_BYTES]);
    for lane in [0, 2, 5, 8, 10] {
        let at = lane * 4;
        let value = f32::from_le_bytes(block[at..at + 4].try_into().expect("4 bytes")) * 2.5;
        block[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    let matrix = shadow_matrix(&block, [10.0, 20.0, 30.0], 1.2);
    let mut moved = [0_u8; ACTOR_TRANSFORM_BLOCK_BYTES];
    moved.copy_from_slice(&matrix[..ACTOR_TRANSFORM_BLOCK_BYTES]);
    let pose = PuppetPose::from_transform_block(&moved);
    assert_eq!(
        pose.position,
        Position {
            x: 10.0,
            y: 20.0,
            z: 30.0
        }
    );
    assert!((pose.yaw - 1.2).abs() < 1e-5);
    let middle = f32::from_le_bytes(matrix[20..24].try_into().expect("4 bytes"));
    assert!((middle - 2.5).abs() < 1e-5);
}

const ENEMY: u64 = 0x5_0000_0000;
const PLAYER: u64 = 0x6_0000_0000;
const PUPPET: u64 = 0x7_0000_0000;
const PROP: u64 = 0x8_0000_0000;

fn put_actor(memory: &mut FakeMemory, actor: u64, slot: u32, kind_mask: u32, serial: i32) {
    memory.put(actor + ACTOR_KIND_MASK_OFFSET, &kind_mask.to_le_bytes());
    let mut handle = [0_u8; 8];
    handle[..4].copy_from_slice(&slot.to_le_bytes());
    handle[4..].copy_from_slice(&1_u32.to_le_bytes());
    memory.put(actor + ACTOR_SELF_HANDLE_OFFSET, &handle);
    memory.put_at_rva(HANDLE_POOL_POINTERS_RVA + slot * 8, &actor.to_le_bytes());
    memory.put_at_rva(HANDLE_POOL_GENERATIONS_RVA + slot * 4, &1_u32.to_le_bytes());
    memory.put(actor + ACTOR_SPAWN_SERIAL_OFFSET, &serial.to_le_bytes());
    let pose = PuppetPose {
        position: Position {
            x: 4.0,
            y: 5.0,
            z: 6.0,
        },
        yaw: 0.25,
    };
    memory.put(
        actor + ACTOR_TRANSFORM_BLOCK_OFFSET,
        &pose.creation_matrix()[..ACTOR_TRANSFORM_BLOCK_BYTES],
    );
}

fn set_group_count(memory: &mut FakeMemory, group: u16, count: i32) {
    let offset = ACTOR_MANAGER_GROUP_COUNT_OFFSET + u64::from(group) * ACTOR_MANAGER_GROUP_STRIDE;
    let at = memory.module_base() + u64::from(ACTOR_MANAGER_RVA) + offset;
    memory.put(at, &count.to_le_bytes());
}

#[test]
fn the_scan_keeps_live_enemies_and_skips_the_player_puppets_and_props() {
    let mut memory = FakeMemory::new();
    let table_bytes = usize::from(ENEMY_GROUP_COUNT) * ACTOR_MANAGER_GROUP_STRIDE as usize;
    memory.put_at_rva(
        u32::try_from(u64::from(ACTOR_MANAGER_RVA) + ACTOR_MANAGER_GROUP_COUNT_OFFSET)
            .expect("rva"),
        &vec![0_u8; table_bytes],
    );
    put_actor(&mut memory, ENEMY, 1, 0x2, 77);
    put_actor(&mut memory, PLAYER, 2, 0x2, 1);
    put_actor(&mut memory, PUPPET, 3, 0x2, 2);
    put_actor(&mut memory, PROP, 4, 0x100, 3);
    let puppet_group = u16::try_from(PUPPET_ACTOR_GROUP).expect("group");
    set_group_count(&mut memory, 6, 3);
    set_group_count(&mut memory, puppet_group, 1);
    let found = scan_enemies(
        &memory,
        Some(PLAYER),
        |group, index| match (group, index) {
            (6, 0) => ENEMY,
            (6, 1) => PLAYER,
            (6, 2) => PROP,
            (group, 0) if group == puppet_group => PUPPET,
            _ => 0,
        },
        |actor| (actor == ENEMY).then_some((30, 40)),
    )
    .expect("scan");
    assert_eq!(found.len(), 1);
    let enemy = found[0];
    assert_eq!(
        (enemy.actor, enemy.group, enemy.spawn_serial),
        (ENEMY, 6, 77)
    );
    assert_eq!(enemy.position, [4.0, 5.0, 6.0]);
    assert!((enemy.yaw_radians - 0.25).abs() < 1e-5);
    assert_eq!((enemy.hp, enemy.max_hp), (30, 40));
}
