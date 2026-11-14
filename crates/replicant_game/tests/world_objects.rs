use replicant_game::GameMemory;
use replicant_game::offsets::{
    ACTOR_CLASS_ID_OFFSET, ACTOR_MANAGER_GROUP_COUNT_OFFSET, ACTOR_MANAGER_GROUP_STRIDE,
    ACTOR_MANAGER_RVA, HANDLE_POOL_GENERATIONS_RVA, HANDLE_POOL_POINTERS_RVA,
    HANDLE_POOL_SLOT_COUNT, PLAYER_ACTION_STATE_OFFSET,
};
use replicant_game::world_objects::{
    ACTION_STATE_TARGET_HANDLE_OFFSET, BOX_MOVE_STATE_VTABLE_RVA, BreakVerdict, HeldTarget,
    HoldHow, MoveVerdict, ObjectSync, PICK_MOVE_STATE_VTABLE_RVA,
    PLAYER_REQUESTED_ACTION_STATE_OFFSET, UNGROUPED_OBJECT_GROUP, WorldObjectRoster,
    box_move_handle_in_running_state, handle_owner, key_text, kind_of_vtable_rva,
    partner_break_verdict, partner_move_verdict, partner_moves_text, read_held_target,
    read_object_sightings, world_object_key, you_move_text,
};
use replicant_proto::ENEMY_GROUP_COUNT;

use crate::fake_memory::{FAKE_MODULE_BASE, FakeMemory};

const CUBE_VTABLE_RVA: u32 = 0x00B0_54B0;
const PUSHED_BOX_VTABLE_RVA: u32 = 0x00B0_0150;
const PUSHED_BOX_ACTOR: u64 = 0xA70C_4D50;
const PUSHED_BOX_HANDLE: u64 = 0x0000_0119_0000_0FC5;
const PUSHED_BOX_PLACE: [f32; 3] = [30.25, 11.95, 66.0];
const ROOM_ONE_CUBE_GROUP: u16 = 11;
const FIRST_CUBE_PLACE: [f32; 3] = [32.25, 11.95, 62.0];
const SECOND_CUBE_PLACE: [f32; 3] = [40.25, 11.95, 72.0];
const PLAYER: u64 = 0x7_0000_0000;
const ACTION_STATE: u64 = 0x7_1000_0000;

fn put_object(memory: &mut FakeMemory, actor: u64, vtable_rva: u32, position: [f32; 3]) {
    memory.put(
        actor,
        &(FAKE_MODULE_BASE + u64::from(vtable_rva)).to_le_bytes(),
    );
    for (offset, axis) in [0x9C_u64, 0xAC, 0xBC].into_iter().zip(position) {
        memory.put(actor + offset, &axis.to_le_bytes());
    }
}

fn put_pool(memory: &mut FakeMemory, actors: &[u64]) {
    let mut table = vec![0_u8; HANDLE_POOL_SLOT_COUNT as usize * 8];
    for (slot, actor) in actors.iter().enumerate() {
        table[slot * 8..slot * 8 + 8].copy_from_slice(&actor.to_le_bytes());
    }
    memory.put_at_rva(HANDLE_POOL_POINTERS_RVA, &table);
}

fn put_group(memory: &mut FakeMemory, group: u16, count: i32) {
    let table_bytes = usize::from(ENEMY_GROUP_COUNT) * ACTOR_MANAGER_GROUP_STRIDE as usize;
    let table = FAKE_MODULE_BASE + u64::from(ACTOR_MANAGER_RVA) + ACTOR_MANAGER_GROUP_COUNT_OFFSET;
    memory.put(table, &vec![0_u8; table_bytes]);
    memory.put(
        table + u64::from(group) * ACTOR_MANAGER_GROUP_STRIDE,
        &count.to_le_bytes(),
    );
}

fn two_cube_game(first: u64, second: u64, pool_order: &[u64]) -> FakeMemory {
    let mut memory = FakeMemory::new();
    put_object(&mut memory, first, CUBE_VTABLE_RVA, FIRST_CUBE_PLACE);
    put_object(&mut memory, second, CUBE_VTABLE_RVA, SECOND_CUBE_PLACE);
    put_pool(&mut memory, pool_order);
    put_group(&mut memory, ROOM_ONE_CUBE_GROUP, 2);
    memory
}

fn put_action_state(memory: &mut FakeMemory, state_vtable_rva: u32, handle: u64) {
    put_action_state_in(memory, PLAYER_ACTION_STATE_OFFSET, state_vtable_rva, handle);
}

fn put_action_state_in(memory: &mut FakeMemory, slot: u64, state_vtable_rva: u32, handle: u64) {
    memory.put(PLAYER + slot, &ACTION_STATE.to_le_bytes());
    memory.put(
        ACTION_STATE,
        &(FAKE_MODULE_BASE + u64::from(state_vtable_rva)).to_le_bytes(),
    );
    memory.put(
        ACTION_STATE + ACTION_STATE_TARGET_HANDLE_OFFSET,
        &handle.to_le_bytes(),
    );
}

#[test]
fn the_attack_box_cubes_are_a_listed_kind_that_breaks() {
    let cube = kind_of_vtable_rva(CUBE_VTABLE_RVA).expect("listed");
    assert_eq!(cube.class_id, 0xB3);
    assert_eq!(cube.sync, ObjectSync::PushAndBreak);
}

#[test]
fn the_pushed_box_the_run_logged_as_not_listed_is_a_listed_pushed_kind() {
    // Run 1114h host 13:19:26.989: "you push handle 0x11900000fc5: actor 0xa70c4d50 vtable rva
    // 0xb00150 class 0xab at (30.250 11.950 66.000), not a listed kind".
    let kind = kind_of_vtable_rva(PUSHED_BOX_VTABLE_RVA).expect("listed");
    assert_eq!((kind.name, kind.class_id), ("CActorBox", 0xAB));
    assert_eq!(kind.sync, ObjectSync::Push);
}

#[test]
fn the_pushed_box_no_script_group_holds_is_found_in_the_pool_and_named_alike_on_both_games() {
    let game_with_the_box = |actor: u64| {
        let mut memory = FakeMemory::new();
        put_object(&mut memory, actor, PUSHED_BOX_VTABLE_RVA, PUSHED_BOX_PLACE);
        put_pool(&mut memory, &[actor]);
        put_group(&mut memory, ROOM_ONE_CUBE_GROUP, 0);
        memory
    };
    let mut host_roster = WorldObjectRoster::default();
    let mut guest_roster = WorldObjectRoster::default();
    host_roster.observe(
        5,
        &read_object_sightings(&game_with_the_box(PUSHED_BOX_ACTOR), |_, _| 0),
    );
    guest_roster.observe(
        5,
        &read_object_sightings(&game_with_the_box(0xA00F_0000), |_, _| 0),
    );
    let key = world_object_key(
        PUSHED_BOX_VTABLE_RVA,
        UNGROUPED_OBJECT_GROUP,
        PUSHED_BOX_PLACE,
    );
    assert_eq!(key_text(&key), "CActorBox group 1023 from (30 12 66)");
    let host_id = host_roster.id_of_key(&key).expect("host knows it");
    let guest_id = guest_roster.id_of_key(&key).expect("guest knows it");
    assert_eq!(host_roster.actor_of(host_id), Some(PUSHED_BOX_ACTOR));
    assert_eq!(guest_roster.actor_of(guest_id), Some(0xA00F_0000));
}

#[test]
fn the_runs_push_resolves_to_the_listed_pushed_box() {
    let mut memory = FakeMemory::new();
    put_object(
        &mut memory,
        PUSHED_BOX_ACTOR,
        PUSHED_BOX_VTABLE_RVA,
        PUSHED_BOX_PLACE,
    );
    memory.put(
        PUSHED_BOX_ACTOR + ACTOR_CLASS_ID_OFFSET,
        &0xAB_i32.to_le_bytes(),
    );
    let mut pointers = vec![0_u8; HANDLE_POOL_SLOT_COUNT as usize * 8];
    pointers[0xFC5 * 8..0xFC5 * 8 + 8].copy_from_slice(&PUSHED_BOX_ACTOR.to_le_bytes());
    memory.put_at_rva(HANDLE_POOL_POINTERS_RVA, &pointers);
    let mut generations = vec![0_u8; HANDLE_POOL_SLOT_COUNT as usize * 4];
    generations[0xFC5 * 4..0xFC5 * 4 + 4].copy_from_slice(&0x119_u32.to_le_bytes());
    memory.put_at_rva(HANDLE_POOL_GENERATIONS_RVA, &generations);
    put_action_state(&mut memory, BOX_MOVE_STATE_VTABLE_RVA, PUSHED_BOX_HANDLE);
    let held = read_held_target(&memory, PLAYER).expect("a push");
    let owner = handle_owner(&memory, held.handle).expect("resolves");
    assert_eq!(owner.actor, PUSHED_BOX_ACTOR);
    assert!(kind_of_vtable_rva(owner.vtable_rva).is_some());
    let mut roster = WorldObjectRoster::default();
    roster.observe(5, &read_object_sightings(&memory, |_, _| 0));
    assert!(roster.id_of_actor(owner.actor).is_some());
}

#[test]
fn only_a_box_move_state_the_game_runs_names_a_box_moved_now() {
    let mut memory = FakeMemory::new();
    put_action_state_in(
        &mut memory,
        PLAYER_REQUESTED_ACTION_STATE_OFFSET,
        BOX_MOVE_STATE_VTABLE_RVA,
        PUSHED_BOX_HANDLE,
    );
    memory.put(PLAYER + PLAYER_ACTION_STATE_OFFSET, &0_u64.to_le_bytes());
    assert_eq!(box_move_handle_in_running_state(&memory, PLAYER), None);
    put_action_state(&mut memory, PICK_MOVE_STATE_VTABLE_RVA, PUSHED_BOX_HANDLE);
    assert_eq!(box_move_handle_in_running_state(&memory, PLAYER), None);
    put_action_state(&mut memory, BOX_MOVE_STATE_VTABLE_RVA, PUSHED_BOX_HANDLE);
    assert_eq!(
        box_move_handle_in_running_state(&memory, PLAYER),
        Some(PUSHED_BOX_HANDLE)
    );
    assert_eq!(box_move_handle_in_running_state(&memory, 0), None);
}

#[test]
fn two_games_name_the_same_cubes_alike_whatever_order_their_pools_hold_them() {
    let host = two_cube_game(0xA_0000, 0xB_0000, &[0xA_0000, 0xB_0000]);
    let guest = two_cube_game(0xC_0000, 0xD_0000, &[0xD_0000, 0xC_0000]);
    let in_group = |first: u64, second: u64| {
        move |group: u16, index: i32| match (group, index) {
            (ROOM_ONE_CUBE_GROUP, 0) => first,
            (ROOM_ONE_CUBE_GROUP, 1) => second,
            _ => 0,
        }
    };
    let mut host_roster = WorldObjectRoster::default();
    let mut guest_roster = WorldObjectRoster::default();
    host_roster.observe(
        5,
        &read_object_sightings(&host, in_group(0xA_0000, 0xB_0000)),
    );
    guest_roster.observe(
        5,
        &read_object_sightings(&guest, in_group(0xD_0000, 0xC_0000)),
    );
    let first_key = world_object_key(CUBE_VTABLE_RVA, ROOM_ONE_CUBE_GROUP, FIRST_CUBE_PLACE);
    assert_eq!(first_key.start_cell, [32, 12, 62]);
    let host_id = host_roster.id_of_key(&first_key).expect("host knows it");
    let guest_id = guest_roster.id_of_key(&first_key).expect("guest knows it");
    assert_eq!(host_roster.actor_of(host_id), Some(0xA_0000));
    assert_eq!(guest_roster.actor_of(guest_id), Some(0xC_0000));
}

#[test]
fn a_cube_that_no_script_group_lists_is_keyed_as_ungrouped() {
    let mut memory = FakeMemory::new();
    put_object(&mut memory, 0xA_0000, CUBE_VTABLE_RVA, FIRST_CUBE_PLACE);
    put_pool(&mut memory, &[0xA_0000]);
    put_group(&mut memory, ROOM_ONE_CUBE_GROUP, 0);
    let sightings = read_object_sightings(&memory, |_, _| 0);
    assert_eq!(sightings.len(), 1);
    assert_eq!(sightings[0].group, UNGROUPED_OBJECT_GROUP);
}

#[test]
fn a_pushed_cube_keeps_the_key_of_its_first_place() {
    let mut memory = two_cube_game(0xA_0000, 0xB_0000, &[0xA_0000, 0xB_0000]);
    let mut roster = WorldObjectRoster::default();
    roster.observe(5, &read_object_sightings(&memory, |_, _| 0));
    let before: Vec<_> = roster.objects().to_vec();
    put_object(
        &mut memory,
        0xA_0000,
        CUBE_VTABLE_RVA,
        [27.537, 11.95, 68.789],
    );
    let added = roster.observe(5, &read_object_sightings(&memory, |_, _| 0));
    assert!(added.is_empty());
    assert_eq!(roster.objects(), before.as_slice());
}

#[test]
fn a_room_reset_gives_the_new_cubes_their_old_keys_and_a_map_change_forgets_them() {
    let first = two_cube_game(0xA_0000, 0xB_0000, &[0xA_0000, 0xB_0000]);
    let mut roster = WorldObjectRoster::default();
    roster.observe(5, &read_object_sightings(&first, |_, _| 0));
    let keys: Vec<_> = roster.objects().iter().map(|object| object.key).collect();
    let reset = two_cube_game(0xE_0000, 0xF_0000, &[0xE_0000, 0xF_0000]);
    let added = roster.observe(5, &read_object_sightings(&reset, |_, _| 0));
    assert_eq!(
        added.iter().map(|object| object.key).collect::<Vec<_>>(),
        keys
    );
    assert_eq!(roster.id_of_actor(0xA_0000), None);
    assert!(roster.observe(6, &[]).is_empty());
    assert!(roster.objects().is_empty());
}

#[test]
fn the_box_move_state_names_the_pushed_object_and_the_pick_move_state_the_carried_one() {
    let mut memory = FakeMemory::new();
    put_action_state(
        &mut memory,
        BOX_MOVE_STATE_VTABLE_RVA,
        0x0005_0000_0000_0FC2,
    );
    assert_eq!(
        read_held_target(&memory, PLAYER),
        Some(HeldTarget {
            handle: 0x0005_0000_0000_0FC2,
            how: HoldHow::Push
        })
    );
    put_action_state(
        &mut memory,
        PICK_MOVE_STATE_VTABLE_RVA,
        0x0005_0000_0000_0FC2,
    );
    assert_eq!(
        read_held_target(&memory, PLAYER).map(|held| held.how),
        Some(HoldHow::Carry)
    );
    put_action_state(&mut memory, 0x00C9_3E98, 0x0005_0000_0000_0FC2);
    assert_eq!(read_held_target(&memory, PLAYER), None);
}

#[test]
fn a_push_the_game_runs_is_read_from_the_current_state_with_the_requested_slot_empty() {
    // The run's push: the move log saw StateActionBoxMove at +0x16200 and the requested slot was empty.
    let mut memory = FakeMemory::new();
    memory.put(
        PLAYER + PLAYER_REQUESTED_ACTION_STATE_OFFSET,
        &0_u64.to_le_bytes(),
    );
    put_action_state(
        &mut memory,
        BOX_MOVE_STATE_VTABLE_RVA,
        0x0005_0000_0000_0FC2,
    );
    assert_eq!(
        read_held_target(&memory, PLAYER).map(|held| (held.handle, held.how)),
        Some((0x0005_0000_0000_0FC2, HoldHow::Push))
    );
}

#[test]
fn a_grab_only_just_asked_for_is_read_from_the_requested_slot() {
    let mut memory = FakeMemory::new();
    memory.put(PLAYER + PLAYER_ACTION_STATE_OFFSET, &0_u64.to_le_bytes());
    put_action_state_in(
        &mut memory,
        PLAYER_REQUESTED_ACTION_STATE_OFFSET,
        PICK_MOVE_STATE_VTABLE_RVA,
        0x0005_0000_0000_0FC2,
    );
    assert_eq!(
        read_held_target(&memory, PLAYER).map(|held| held.how),
        Some(HoldHow::Carry)
    );
}

#[test]
fn a_held_handle_resolves_through_the_pool_only_while_its_generation_matches() {
    let mut memory = FakeMemory::new();
    put_object(&mut memory, 0xA_0000, CUBE_VTABLE_RVA, FIRST_CUBE_PLACE);
    memory.put(0xA_0000 + ACTOR_CLASS_ID_OFFSET, &0xB3_i32.to_le_bytes());
    let mut pointers = vec![0_u8; HANDLE_POOL_SLOT_COUNT as usize * 8];
    pointers[0xFC2 * 8..0xFC2 * 8 + 8].copy_from_slice(&0xA_0000_u64.to_le_bytes());
    memory.put_at_rva(HANDLE_POOL_POINTERS_RVA, &pointers);
    let mut generations = vec![0_u8; HANDLE_POOL_SLOT_COUNT as usize * 4];
    generations[0xFC2 * 4..0xFC2 * 4 + 4].copy_from_slice(&0x5E1_u32.to_le_bytes());
    memory.put_at_rva(HANDLE_POOL_GENERATIONS_RVA, &generations);
    let owner = handle_owner(&memory, 0x5E1_0000_0FC2).expect("resolves");
    assert_eq!(owner.actor, 0xA_0000);
    assert_eq!(owner.vtable_rva, CUBE_VTABLE_RVA);
    assert_eq!(owner.class_id, 0xB3);
    assert_eq!(handle_owner(&memory, 0x5E2_0000_0FC2), None);
    assert!(memory.read_u64(0xA_0000).is_ok());
}

#[test]
fn the_host_keeps_a_cube_it_pushes_when_the_guests_move_crosses() {
    assert_eq!(
        partner_move_verdict(true, true, true),
        MoveVerdict::KeepHostsAndAnswer
    );
    assert_eq!(partner_move_verdict(true, false, true), MoveVerdict::Follow);
}

#[test]
fn the_guest_follows_the_host_even_while_both_push_the_same_cube() {
    assert_eq!(partner_move_verdict(false, true, true), MoveVerdict::Follow);
    assert_eq!(
        partner_move_verdict(false, false, false),
        MoveVerdict::Follow
    );
}

#[test]
fn a_guest_pushing_a_cube_the_host_lets_be_keeps_its_push_over_the_hosts_resend() {
    assert_eq!(
        partner_move_verdict(false, true, false),
        MoveVerdict::KeepOwnPush
    );
}

#[test]
fn a_partners_break_waits_for_a_cube_that_is_not_hittable_yet() {
    assert_eq!(partner_break_verdict(true, true), BreakVerdict::BreakHere);
    assert_eq!(
        partner_break_verdict(true, false),
        BreakVerdict::WaitUntilHittable
    );
    assert_eq!(partner_break_verdict(false, true), BreakVerdict::NotHere);
}

#[test]
fn a_key_reads_as_its_kind_group_and_first_cell() {
    let key = world_object_key(CUBE_VTABLE_RVA, ROOM_ONE_CUBE_GROUP, FIRST_CUBE_PLACE);
    assert_eq!(key_text(&key), "CActorAttackBox group 11 from (32 12 62)");
}

#[test]
fn a_moving_pushed_box_logs_each_second_where_both_copies_are() {
    let key = world_object_key(
        PUSHED_BOX_VTABLE_RVA,
        UNGROUPED_OBJECT_GROUP,
        PUSHED_BOX_PLACE,
    );
    assert_eq!(
        you_move_text(&key, HoldHow::Push, Some([29.5, 11.95, 66.0]), None),
        "objects: you push CActorBox group 1023 from (30 12 66): your copy at (29.500 11.950 66.000), the partner's copy last heard at unknown"
    );
    assert_eq!(
        partner_moves_text(&key, [29.5, 11.95, 66.0], Some([29.75, 11.95, 66.0])),
        "objects: the partner moves CActorBox group 1023 from (30 12 66): their copy at (29.500 11.950 66.000), yours at (29.750 11.950 66.000)"
    );
}
