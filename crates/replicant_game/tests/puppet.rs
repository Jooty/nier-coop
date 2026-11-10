use replicant_game::Position;
use replicant_game::offsets::{
    ACTOR_MOTION_PLAYER_OFFSET, ACTOR_SELF_HANDLE_OFFSET, HANDLE_POOL_GENERATIONS_RVA,
    HANDLE_POOL_POINTERS_RVA, MOTION_PLAYER_LAYER_COUNT_OFFSET,
    MOTION_PLAYER_LAYER_POINTERS_OFFSET, MOTION_PLAYER_PACK_LIST_OFFSET,
};
use replicant_game::puppet::{
    LayerMotion, MOTION_BLEND_SECONDS, MotionEntry, MotionStep, PuppetPose, actor_is_live,
    follow_step, frame_to_seek, motion_entries, motion_entry_for_hash, motion_name,
    read_layer_motion, read_puppet_layer, seek_layer_frame, wrapped_frame,
};

use replicant_proto::motion_name_hash;

use crate::fake_memory::FakeMemory;

const ACTOR: u64 = 0x2_0000_0000;
const LAYER: u64 = 0x3_0000_0000;
const SLOTS: u64 = 0x3_1000_0000;
const NODE: u64 = 0x4_0000_0000;
const PACK: u64 = 0x4_1000_0000;
const NAMED_MOTION_INDEX: i32 = 1;
const NAMED_MOTION: &str = "NIER013_wait";

fn memory_with_player_motion() -> FakeMemory {
    let mut memory = FakeMemory::new();
    let player = ACTOR + ACTOR_MOTION_PLAYER_OFFSET;
    memory.put(
        player + MOTION_PLAYER_LAYER_COUNT_OFFSET,
        &6_i32.to_le_bytes(),
    );
    memory.put(
        player + MOTION_PLAYER_LAYER_POINTERS_OFFSET,
        &LAYER.to_le_bytes(),
    );
    let mut layer = vec![0_u8; 0x20];
    layer[0x8..0xC].copy_from_slice(&1_i32.to_le_bytes());
    layer[0xC..0x10].copy_from_slice(&3_i32.to_le_bytes());
    layer[0x10..0x18].copy_from_slice(&SLOTS.to_le_bytes());
    memory.put(LAYER, &layer);
    let mut slot = vec![0_u8; 0x258];
    slot[0..4].copy_from_slice(&388_i32.to_le_bytes());
    slot[4..8].copy_from_slice(&7.5_f32.to_le_bytes());
    slot[0x10..0x14].copy_from_slice(&1.0_f32.to_le_bytes());
    slot[0x250] = 1;
    let mut slots = vec![0_u8; 0x258];
    slots.extend_from_slice(&slot);
    slots.extend_from_slice(&[0xFF; 0x258]);
    memory.put(SLOTS, &slots);
    memory.put(
        player + MOTION_PLAYER_PACK_LIST_OFFSET + 0x10,
        &NODE.to_le_bytes(),
    );
    memory.put(NODE + 0x18, &PACK.to_le_bytes());
    memory.put(NODE + 0x28, &0_u64.to_le_bytes());
    let mut pack = vec![0_u8; 0x100];
    pack[0..4].copy_from_slice(b"KPK\x7f");
    pack[4..8].copy_from_slice(&2_u32.to_le_bytes());
    pack[0x0C..0x10].copy_from_slice(&0x40_u32.to_le_bytes());
    pack[0x10..0x14].copy_from_slice(&0x80_u32.to_le_bytes());
    pack[0x80..0x83].copy_from_slice(b"CMF");
    pack[0x8C..0x8C + NAMED_MOTION.len()].copy_from_slice(NAMED_MOTION.as_bytes());
    memory.put(PACK, &pack);
    memory
}

#[test]
fn pose_round_trips_through_the_creation_matrix() {
    let pose = PuppetPose {
        position: Position {
            x: 10.0,
            y: 2.5,
            z: -4.0,
        },
        yaw: -0.84,
    };
    let mut block = [0_u8; 48];
    block.copy_from_slice(&pose.creation_matrix()[..48]);
    let read_back = PuppetPose::from_transform_block(&block);
    assert_eq!(read_back.position, pose.position);
    assert!((read_back.yaw - pose.yaw).abs() < 1e-5);
}

#[test]
fn creation_matrix_ends_with_the_homogeneous_row() {
    let matrix = PuppetPose::default().creation_matrix();
    assert_eq!(&matrix[48..52], &0.0_f32.to_le_bytes());
    assert_eq!(&matrix[60..64], &1.0_f32.to_le_bytes());
}

#[test]
fn layer_motion_reads_the_current_slot() {
    let memory = memory_with_player_motion();
    let motion = read_layer_motion(&memory, ACTOR, 0)
        .expect("readable")
        .expect("a motion is playing");
    assert_eq!(motion.index, 388);
    assert_eq!(motion.frame, 7.5);
    assert_eq!(motion.speed, 1.0);
    assert!(motion.looping);
}

#[test]
fn layer_beyond_the_count_has_no_motion() {
    let memory = memory_with_player_motion();
    assert_eq!(read_layer_motion(&memory, ACTOR, 6), Ok(None));
}

#[test]
fn motion_name_comes_from_the_pack_list() {
    let memory = memory_with_player_motion();
    assert_eq!(
        motion_name(&memory, ACTOR, NAMED_MOTION_INDEX),
        Ok(Some(NAMED_MOTION.to_owned()))
    );
    assert_eq!(motion_name(&memory, ACTOR, 2), Ok(None));
    assert_eq!(motion_name(&memory, ACTOR, -1), Ok(None));
}

#[test]
fn actor_is_live_only_while_the_handle_pool_still_points_at_it() {
    let mut memory = FakeMemory::new();
    let mut handle = 7_u32.to_le_bytes().to_vec();
    handle.extend_from_slice(&3_u32.to_le_bytes());
    memory.put(ACTOR + ACTOR_SELF_HANDLE_OFFSET, &handle);
    memory.put_at_rva(HANDLE_POOL_POINTERS_RVA + 7 * 8, &ACTOR.to_le_bytes());
    memory.put_at_rva(HANDLE_POOL_GENERATIONS_RVA + 7 * 4, &3_u32.to_le_bytes());
    assert_eq!(actor_is_live(&memory, ACTOR), Ok(true));
    memory.put_at_rva(HANDLE_POOL_GENERATIONS_RVA + 7 * 4, &4_u32.to_le_bytes());
    assert_eq!(actor_is_live(&memory, ACTOR), Ok(false));
    memory.put_at_rva(HANDLE_POOL_GENERATIONS_RVA + 7 * 4, &3_u32.to_le_bytes());
    memory.put_at_rva(HANDLE_POOL_POINTERS_RVA + 7 * 8, &0_u64.to_le_bytes());
    assert_eq!(actor_is_live(&memory, ACTOR), Ok(false));
}

fn layer(index: i32, frame: f32) -> LayerMotion {
    LayerMotion {
        index,
        frame,
        speed: 1.0,
        looping: true,
    }
}

#[test]
fn an_unreadable_puppet_layer_starts_the_motion_once() {
    assert_eq!(
        follow_step(None, None, 1037),
        MotionStep::Start {
            blend: MOTION_BLEND_SECONDS
        }
    );
    assert_eq!(follow_step(None, Some(1037), 1037), MotionStep::Keep);
}

#[test]
fn a_new_remote_motion_starts_with_a_blend() {
    assert_eq!(
        follow_step(Some(layer(388, 10.0)), Some(388), 1037),
        MotionStep::Start {
            blend: MOTION_BLEND_SECONDS
        }
    );
}

#[test]
fn a_running_motion_is_never_restarted_for_drift() {
    assert_eq!(
        follow_step(Some(layer(1037, 10.0)), Some(1037), 1037),
        MotionStep::Keep
    );
}

#[test]
fn a_puppet_within_half_a_frame_is_not_seeked() {
    assert_eq!(frame_to_seek(10.0, 10.4, 61, true), None);
    assert_eq!(frame_to_seek(10.0, 11.0, 61, true), Some(11.0));
}

#[test]
fn a_puppet_that_does_not_advance_is_seeked_every_frame() {
    let mut on_puppet = 10.0;
    for tick in 1..=5 {
        let target = 10.0 + tick as f32;
        assert_eq!(frame_to_seek(on_puppet, target, 61, true), Some(target));
        on_puppet = target;
    }
}

#[test]
fn a_loop_is_compared_across_its_seam_and_targets_wrap() {
    assert_eq!(frame_to_seek(59.9, 60.1, 61, true), None);
    assert_eq!(frame_to_seek(5.0, 62.0, 61, true), Some(2.0));
    assert_eq!(wrapped_frame(-1.0, 61, true), 59.0);
}

#[test]
fn a_one_shot_motion_holds_its_last_frame() {
    assert_eq!(wrapped_frame(80.0, 61, false), 60.0);
    assert_eq!(frame_to_seek(10.0, 80.0, 61, false), Some(60.0));
    assert_eq!(wrapped_frame(f32::NAN, 61, false), 0.0);
}

#[test]
fn seeking_writes_the_current_and_previous_frame_of_the_slot() {
    let mut memory = memory_with_player_motion();
    let layer = read_puppet_layer(&memory, ACTOR, 0)
        .expect("readable")
        .expect("a motion is playing");
    assert_eq!(layer.frame_count, 0);
    seek_layer_frame(&mut memory, &layer, 21.5).expect("writable");
    let motion = read_layer_motion(&memory, ACTOR, 0)
        .expect("readable")
        .expect("a motion is playing");
    assert_eq!(motion.frame, 21.5);
    assert_eq!(memory.get(layer.slot + 8, 4), 21.5_f32.to_le_bytes());
}

#[test]
fn a_read_back_that_disagrees_with_a_started_motion_does_not_restart_it() {
    assert_eq!(
        follow_step(Some(layer(0, 0.0)), Some(1037), 1037),
        MotionStep::Keep
    );
}

#[test]
fn motion_entries_list_every_named_motion_with_its_index() {
    let memory = memory_with_player_motion();
    let entries = motion_entries(&memory, ACTOR).expect("pack list readable");
    assert_eq!(
        entries,
        vec![MotionEntry {
            index: NAMED_MOTION_INDEX,
            name: NAMED_MOTION.to_owned()
        }]
    );
}

#[test]
fn a_motion_is_found_on_another_body_by_its_name_hash() {
    let memory = memory_with_player_motion();
    let entries = motion_entries(&memory, ACTOR).expect("pack list readable");
    let found = motion_entry_for_hash(&entries, motion_name_hash(NAMED_MOTION));
    assert_eq!(found.map(|entry| entry.index), Some(NAMED_MOTION_INDEX));
    assert_eq!(
        motion_entry_for_hash(&entries, motion_name_hash("NIER013_missing")),
        None
    );
}
