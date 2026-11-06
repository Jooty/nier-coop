use rc_game::Position;
use rc_game::marker::{CONTROL_MASK_FIRST_RVA, EVENT_SCENE_BYTE_RVA};
use rc_game::npcs::{
    MirroredNpc, NPC_LOCAL_ONLY_GRACE_MS, NPC_MIRROR_STALE_MS, NpcHoldBlocker, NpcMirror,
    NpcPlanner, NpcRoster, NpcSighting, npc_state, read_npc_hold_blocker, scan_npcs,
    split_npc_snapshot,
};
use rc_game::offsets::{
    ACTOR_CLASS_ID_OFFSET, ACTOR_DRAWN_FLAG_OFFSET, ACTOR_LIFE_STATE_OFFSET, ACTOR_MANAGER_RVA,
    ACTOR_MANAGER_SECONDARY_LIST_HEAD_OFFSET, ACTOR_SCRIPT_GROUP_OFFSET,
    ACTOR_SECONDARY_LIST_NEXT_OFFSET, ACTOR_SELF_HANDLE_OFFSET, ACTOR_SPAWN_SERIAL_OFFSET,
    ACTOR_TRANSFORM_BLOCK_BYTES, ACTOR_TRANSFORM_BLOCK_OFFSET, HANDLE_POOL_GENERATIONS_RVA,
    HANDLE_POOL_POINTERS_RVA, HUMAN_NPC_VTABLE_RVA, TALK_CONTROL_MASK_SLOT,
};
use rc_game::puppet::PuppetPose;
use rc_proto::{
    MAX_NPC_SNAPSHOT_PARTS, MAX_NPCS_PER_SNAPSHOT, Message, MotionState, NPC_UNGROUPED, NpcId,
    NpcSnapshot, NpcState, encode,
};

use crate::fake_memory::{FAKE_MODULE_BASE, FakeMemory};

const PHASE: u16 = 12;
const VILLAGER_CLASS: u16 = 104;
const MERCHANT_CLASS: u16 = 110;

fn sighting(actor: u64, group: u16, class: u16, serial: i32) -> NpcSighting {
    NpcSighting {
        actor,
        spawn_serial: serial,
        group,
        class,
        position: [actor as f32, 0.0, 0.0],
        yaw_radians: 0.5,
        motion: None,
        visible: true,
    }
}

fn id(group: u16, class: u16, ordinal: u16) -> NpcId {
    NpcId {
        group,
        class,
        ordinal,
    }
}

fn state(npc: NpcId, x: f32, frame: f32) -> NpcState {
    NpcState {
        id: npc,
        position: [x, 0.0, 0.0],
        yaw_radians: 0.0,
        motion: Some(MotionState {
            index: 3,
            frame,
            speed: 1.0,
            looping: true,
        }),
        visible: true,
    }
}

fn snapshot(tick: u32, phase: u16, npcs: Vec<NpcState>) -> NpcSnapshot {
    NpcSnapshot {
        tick,
        phase,
        part: 0,
        parts: 1,
        total: u16::try_from(npcs.len()).expect("small"),
        npcs,
    }
}

fn mirrored(npc: NpcId) -> MirroredNpc {
    MirroredNpc {
        id: npc,
        position: [0.0; 3],
        yaw_radians: 0.0,
        motion: None,
        visible: true,
    }
}

#[test]
fn ordinals_count_per_group_and_class_in_spawn_order() {
    let mut roster = NpcRoster::new();
    let update = roster.observe(
        PHASE,
        &[
            sighting(0x300, 4, VILLAGER_CLASS, 9),
            sighting(0x100, 4, VILLAGER_CLASS, 2),
            sighting(0x200, 4, MERCHANT_CLASS, 5),
            sighting(0x400, NPC_UNGROUPED, VILLAGER_CLASS, 1),
        ],
    );
    let by_actor = |actor: u64| {
        update
            .seen
            .iter()
            .find(|(_, seen)| seen.actor == actor)
            .map(|(npc, _)| *npc)
    };
    assert_eq!(by_actor(0x100), Some(id(4, VILLAGER_CLASS, 0)));
    assert_eq!(by_actor(0x300), Some(id(4, VILLAGER_CLASS, 1)));
    assert_eq!(by_actor(0x200), Some(id(4, MERCHANT_CLASS, 0)));
    assert_eq!(by_actor(0x400), Some(id(NPC_UNGROUPED, VILLAGER_CLASS, 0)));
    assert_eq!(update.arrived.len(), 4);
}

#[test]
fn an_extra_npc_of_one_model_leaves_the_other_models_numbers_alone() {
    let mut host = NpcRoster::new();
    let mut guest = NpcRoster::new();
    let host_update = host.observe(
        PHASE,
        &[
            sighting(0x100, 4, VILLAGER_CLASS, 1),
            sighting(0x200, 4, MERCHANT_CLASS, 3),
        ],
    );
    let guest_update = guest.observe(
        PHASE,
        &[
            sighting(0x900, 4, VILLAGER_CLASS, 1),
            sighting(0x950, 4, VILLAGER_CLASS, 2),
            sighting(0x800, 4, MERCHANT_CLASS, 3),
        ],
    );
    let merchant = |update: &rc_game::npcs::NpcRosterUpdate| {
        update
            .seen
            .iter()
            .find(|(npc, _)| npc.class == MERCHANT_CLASS)
            .map(|(npc, _)| *npc)
    };
    assert_eq!(merchant(&host_update), merchant(&guest_update));
}

#[test]
fn a_known_npc_keeps_its_id_and_a_vanished_one_is_reported_gone() {
    let mut roster = NpcRoster::new();
    roster.observe(
        PHASE,
        &[
            sighting(0x100, 4, VILLAGER_CLASS, 1),
            sighting(0x200, 4, VILLAGER_CLASS, 2),
        ],
    );
    let update = roster.observe(PHASE, &[sighting(0x200, 4, VILLAGER_CLASS, 2)]);
    assert_eq!(update.seen[0].0, id(4, VILLAGER_CLASS, 1));
    assert_eq!(update.gone, vec![id(4, VILLAGER_CLASS, 0)]);
    assert!(update.arrived.is_empty());
}

#[test]
fn a_new_phase_numbers_from_zero_again() {
    let mut roster = NpcRoster::new();
    roster.observe(PHASE, &[sighting(0x100, 4, VILLAGER_CLASS, 1)]);
    let update = roster.observe(PHASE + 1, &[sighting(0x500, 4, VILLAGER_CLASS, 7)]);
    assert!(update.phase_changed);
    assert_eq!(update.seen[0].0, id(4, VILLAGER_CLASS, 0));
}

#[test]
fn the_mirror_replays_the_host_between_two_snapshots() {
    let villager = id(4, VILLAGER_CLASS, 0);
    let mut mirror = NpcMirror::new();
    mirror.receive_snapshot(1000, &snapshot(1, PHASE, vec![state(villager, 0.0, 10.0)]));
    mirror.receive_snapshot(1100, &snapshot(2, PHASE, vec![state(villager, 10.0, 16.0)]));
    let sampled = mirror.sample(1150, 100);
    assert_eq!(sampled.len(), 1);
    assert!((sampled[0].position[0] - 5.0).abs() < 1e-4);
    let frame = sampled[0].motion.expect("motion").frame;
    assert!((frame - 13.0).abs() < 1e-4);
}

#[test]
fn the_mirror_forgets_npcs_the_host_stopped_sending() {
    let villager = id(4, VILLAGER_CLASS, 0);
    let mut mirror = NpcMirror::new();
    mirror.receive_snapshot(1000, &snapshot(1, PHASE, vec![state(villager, 0.0, 0.0)]));
    assert!(mirror.is_current(1000 + NPC_MIRROR_STALE_MS));
    mirror.prune(1001 + NPC_MIRROR_STALE_MS);
    assert!(mirror.is_empty());
    assert!(!mirror.is_current(1001 + NPC_MIRROR_STALE_MS));
}

#[test]
fn a_snapshot_for_another_phase_starts_the_mirror_over() {
    let mut mirror = NpcMirror::new();
    mirror.receive_snapshot(
        1000,
        &snapshot(1, PHASE, vec![state(id(4, VILLAGER_CLASS, 0), 0.0, 0.0)]),
    );
    mirror.receive_snapshot(
        1100,
        &snapshot(
            2,
            PHASE + 1,
            vec![state(id(5, VILLAGER_CLASS, 0), 0.0, 0.0)],
        ),
    );
    assert_eq!(mirror.phase(), Some(PHASE + 1));
    assert_eq!(mirror.len(), 1);
}

#[test]
fn matched_npcs_are_driven_at_once() {
    let villager = id(4, VILLAGER_CLASS, 0);
    let mut planner = NpcPlanner::new();
    let plan = planner.plan(0, &[villager], &[mirrored(villager)], true);
    assert_eq!(plan.drive, vec![mirrored(villager)]);
    assert!(plan.hide.is_empty() && plan.missing_here.is_empty());
}

#[test]
fn a_local_only_npc_is_hidden_only_after_the_grace_time() {
    let extra = id(4, VILLAGER_CLASS, 3);
    let mut planner = NpcPlanner::new();
    let first = planner.plan(1000, &[extra], &[], true);
    assert_eq!(first.waiting, vec![extra]);
    assert!(first.hide.is_empty());
    let later = planner.plan(1000 + NPC_LOCAL_ONLY_GRACE_MS, &[extra], &[], true);
    assert_eq!(later.hide, vec![extra]);
    assert_eq!(later.newly_hidden, vec![extra]);
    let after = planner.plan(2000 + NPC_LOCAL_ONLY_GRACE_MS, &[extra], &[], true);
    assert_eq!(after.hide, vec![extra]);
    assert!(after.newly_hidden.is_empty());
}

#[test]
fn nothing_is_hidden_while_the_host_list_is_not_current() {
    let extra = id(4, VILLAGER_CLASS, 3);
    let mut planner = NpcPlanner::new();
    planner.plan(0, &[extra], &[], false);
    let plan = planner.plan(NPC_LOCAL_ONLY_GRACE_MS * 2, &[extra], &[], false);
    assert!(plan.hide.is_empty() && plan.waiting.is_empty() && plan.missing_here.is_empty());
}

#[test]
fn a_host_npc_missing_here_is_reported_once() {
    let host_only = id(9, MERCHANT_CLASS, 0);
    let mut planner = NpcPlanner::new();
    let first = planner.plan(0, &[], &[mirrored(host_only)], true);
    assert_eq!(first.missing_here, vec![host_only]);
    assert_eq!(first.newly_missing, vec![host_only]);
    let second = planner.plan(100, &[], &[mirrored(host_only)], true);
    assert_eq!(second.missing_here, vec![host_only]);
    assert!(second.newly_missing.is_empty());
}

#[test]
fn an_npc_that_matches_again_is_no_longer_hidden() {
    let npc = id(4, VILLAGER_CLASS, 0);
    let mut planner = NpcPlanner::new();
    planner.plan(0, &[npc], &[], true);
    let hidden = planner.plan(NPC_LOCAL_ONLY_GRACE_MS, &[npc], &[], true);
    assert_eq!(hidden.hide, vec![npc]);
    let back = planner.plan(NPC_LOCAL_ONLY_GRACE_MS + 1, &[npc], &[mirrored(npc)], true);
    assert!(back.hide.is_empty());
    assert_eq!(back.drive.len(), 1);
}

#[test]
fn a_crowded_snapshot_sends_the_nearest_npcs_and_every_part_fits() {
    let count = MAX_NPCS_PER_SNAPSHOT * usize::from(MAX_NPC_SNAPSHOT_PARTS) + 3;
    let npcs: Vec<NpcState> = (0..count)
        .map(|n| {
            let ordinal = u16::try_from(n).expect("small");
            state(id(4, VILLAGER_CLASS, ordinal), n as f32 * 10.0, 0.0)
        })
        .collect();
    let parts = split_npc_snapshot(1, PHASE, npcs, &[[0.0; 3]]);
    assert_eq!(parts.len(), usize::from(MAX_NPC_SNAPSHOT_PARTS));
    let sent: Vec<NpcId> = parts
        .iter()
        .flat_map(|part| part.npcs.iter().map(|npc| npc.id))
        .collect();
    assert_eq!(sent[0], id(4, VILLAGER_CLASS, 0));
    assert!(!sent.contains(&id(
        4,
        VILLAGER_CLASS,
        u16::try_from(count - 1).expect("small")
    )));
    for part in parts {
        assert_eq!(usize::from(part.total), count);
        encode(&Message::NpcSnapshot(part)).expect("every part is a valid message");
    }
}

#[test]
fn an_empty_scene_still_sends_one_part() {
    let parts = split_npc_snapshot(1, PHASE, Vec::new(), &[[0.0; 3]]);
    assert_eq!(parts.len(), 1);
    assert!(parts[0].npcs.is_empty());
}

#[test]
fn a_sighting_with_an_unusable_position_is_not_sent() {
    let mut bad = sighting(0x100, 4, VILLAGER_CLASS, 1);
    bad.position[1] = f32::NAN;
    assert!(npc_state(id(4, VILLAGER_CLASS, 0), &bad).is_none());
    let mut hidden = sighting(0x100, 4, VILLAGER_CLASS, 1);
    hidden.visible = false;
    let sent = npc_state(id(4, VILLAGER_CLASS, 0), &hidden).expect("usable");
    assert!(!sent.visible);
}

const VILLAGER: u64 = 0x5_0000_0000;
const LONER: u64 = 0x5_1000_0000;
const DYING: u64 = 0x5_2000_0000;
const ENEMY: u64 = 0x5_3000_0000;

fn put_list_actor(
    memory: &mut FakeMemory,
    actor: u64,
    slot: u32,
    vtable: u64,
    next: u64,
    group: i32,
    life_state: i32,
) {
    memory.put(actor, &vtable.to_le_bytes());
    memory.put(
        actor + ACTOR_SECONDARY_LIST_NEXT_OFFSET,
        &next.to_le_bytes(),
    );
    let mut handle = [0_u8; 8];
    handle[..4].copy_from_slice(&slot.to_le_bytes());
    handle[4..].copy_from_slice(&1_u32.to_le_bytes());
    memory.put(actor + ACTOR_SELF_HANDLE_OFFSET, &handle);
    memory.put_at_rva(HANDLE_POOL_POINTERS_RVA + slot * 8, &actor.to_le_bytes());
    memory.put_at_rva(HANDLE_POOL_GENERATIONS_RVA + slot * 4, &1_u32.to_le_bytes());
    memory.put(actor + ACTOR_LIFE_STATE_OFFSET, &life_state.to_le_bytes());
    memory.put(actor + ACTOR_SCRIPT_GROUP_OFFSET, &group.to_le_bytes());
    memory.put(
        actor + ACTOR_CLASS_ID_OFFSET,
        &i32::from(VILLAGER_CLASS).to_le_bytes(),
    );
    memory.put(
        actor + ACTOR_SPAWN_SERIAL_OFFSET,
        &i32::try_from(slot).expect("small").to_le_bytes(),
    );
    memory.put(actor + ACTOR_DRAWN_FLAG_OFFSET, &[1]);
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

#[test]
fn the_scan_walks_the_actor_list_and_keeps_live_villagers_only() {
    let mut memory = FakeMemory::new();
    let npc_vtable = FAKE_MODULE_BASE + u64::from(HUMAN_NPC_VTABLE_RVA);
    let head =
        u32::try_from(u64::from(ACTOR_MANAGER_RVA) + ACTOR_MANAGER_SECONDARY_LIST_HEAD_OFFSET)
            .expect("rva");
    memory.put_at_rva(head, &ENEMY.to_le_bytes());
    put_list_actor(&mut memory, ENEMY, 1, npc_vtable + 0x100, VILLAGER, 3, 0);
    put_list_actor(&mut memory, VILLAGER, 2, npc_vtable, DYING, 7, 0);
    put_list_actor(&mut memory, DYING, 3, npc_vtable, LONER, 7, 5);
    put_list_actor(&mut memory, LONER, 4, npc_vtable, 0, -1, 0);
    let found = scan_npcs(&memory).expect("scan");
    let actors: Vec<(u64, u16)> = found.iter().map(|npc| (npc.actor, npc.group)).collect();
    assert_eq!(actors, vec![(VILLAGER, 7), (LONER, NPC_UNGROUPED)]);
    assert_eq!(found[0].class, VILLAGER_CLASS);
    assert_eq!(found[0].position, [4.0, 5.0, 6.0]);
    assert!(found[0].visible);
}

#[test]
fn a_looping_actor_list_ends_the_scan() {
    let mut memory = FakeMemory::new();
    let npc_vtable = FAKE_MODULE_BASE + u64::from(HUMAN_NPC_VTABLE_RVA);
    let head =
        u32::try_from(u64::from(ACTOR_MANAGER_RVA) + ACTOR_MANAGER_SECONDARY_LIST_HEAD_OFFSET)
            .expect("rva");
    memory.put_at_rva(head, &VILLAGER.to_le_bytes());
    put_list_actor(&mut memory, VILLAGER, 2, npc_vtable, VILLAGER, 7, 0);
    let found = scan_npcs(&memory).expect("scan ends");
    assert!(!found.is_empty());
}

fn put_hold_flags(memory: &mut FakeMemory, event_scene: u8, talk: i32) {
    memory.put_at_rva(EVENT_SCENE_BYTE_RVA, &[event_scene]);
    memory.put_at_rva(
        CONTROL_MASK_FIRST_RVA + TALK_CONTROL_MASK_SLOT * 4,
        &talk.to_le_bytes(),
    );
}

#[test]
fn npcs_are_held_only_outside_event_scenes_and_conversations() {
    let mut memory = FakeMemory::new();
    put_hold_flags(&mut memory, 0, 0);
    assert_eq!(read_npc_hold_blocker(&memory), Ok(None));
    put_hold_flags(&mut memory, 0, 0xA0);
    assert_eq!(
        read_npc_hold_blocker(&memory),
        Ok(Some(NpcHoldBlocker::Talk))
    );
    put_hold_flags(&mut memory, 1, 0xA0);
    assert_eq!(
        read_npc_hold_blocker(&memory),
        Ok(Some(NpcHoldBlocker::EventScene))
    );
}
