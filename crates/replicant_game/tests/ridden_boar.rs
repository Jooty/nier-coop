use std::f32::consts::FRAC_PI_2;

use replicant_game::Position;
use replicant_game::offsets::{
    ACTOR_CLASS_ID_OFFSET, ACTOR_NEW_MOTION_PLAYER_FLAG_OFFSET,
    ACTOR_OLD_MOTION_LAYER_COUNT_OFFSET, ACTOR_OLD_MOTION_LAYER_POINTERS_OFFSET,
    ACTOR_SELF_HANDLE_OFFSET, ACTOR_TRANSFORM_BLOCK_OFFSET, HANDLE_POOL_GENERATIONS_RVA,
    HANDLE_POOL_POINTERS_RVA, HANDLE_POOL_SLOT_COUNT, PLAYER_ACTION_STATE_OFFSET,
    PUPPET_FALLBACK_CLASS_ID, RIDDEN_BOAR_CLASS_ID, RIDE_BOAR_STATE_BOAR_HANDLE_OFFSET,
    RIDE_BOAR_STATE_VTABLE_RVA,
};
use replicant_game::puppet::{
    BoarPackSettle, BoarStep, PuppetPose, RIDDEN_BOAR_PACK_LOOKUP_INTERVAL_MS,
    RIDDEN_BOAR_PACK_SETTLE_MS, boar_pose_under_rider, boar_seen_from_rider, boar_step,
    read_ridden_boar, read_ridden_boar_actor,
};
use replicant_proto::{NO_MOTION_NAME_HASH, RiddenBoar};

use crate::fake_memory::{FAKE_MODULE_BASE, FakeMemory};

const PLAYER: u64 = 0x3_0000_0000;
const RIDE_STATE: u64 = 0x3_1000_0000;
const BOAR: u64 = 0x3_2000_0000;
const BOAR_LAYER: u64 = 0x3_3000_0000;
const BOAR_SLOTS: u64 = 0x3_4000_0000;
const BOAR_POOL_SLOT: u32 = 77;
const BOAR_GENERATION: u32 = 3;
const SOME_OTHER_STATE_VTABLE_RVA: u32 = 0xC9_0000;
const WALK_MOTION: i32 = 7;

fn pose(x: f32, y: f32, z: f32, yaw: f32) -> PuppetPose {
    PuppetPose {
        position: Position { x, y, z },
        yaw,
    }
}

fn put_pose(memory: &mut FakeMemory, actor: u64, pose: PuppetPose) {
    memory.put(
        actor + ACTOR_TRANSFORM_BLOCK_OFFSET,
        &pose.creation_matrix()[..48],
    );
}

fn put_handle(memory: &mut FakeMemory, at: u64, index: u32, generation: u32) {
    memory.put(at, &index.to_le_bytes());
    memory.put(at + 4, &generation.to_le_bytes());
}

fn put_pool_slot(memory: &mut FakeMemory, index: u32, generation: u32, actor: u64) {
    memory.put_at_rva(
        HANDLE_POOL_GENERATIONS_RVA + index * 4,
        &generation.to_le_bytes(),
    );
    memory.put_at_rva(HANDLE_POOL_POINTERS_RVA + index * 8, &actor.to_le_bytes());
}

fn put_walking_body_layer(memory: &mut FakeMemory, frame: f32) {
    memory.put(
        BOAR + ACTOR_OLD_MOTION_LAYER_COUNT_OFFSET,
        &1_i32.to_le_bytes(),
    );
    memory.put(
        BOAR + ACTOR_OLD_MOTION_LAYER_POINTERS_OFFSET,
        &BOAR_LAYER.to_le_bytes(),
    );
    memory.put(BOAR_LAYER, &1_u64.to_le_bytes());
    memory.put(BOAR_LAYER + 0x8, &0_i32.to_le_bytes());
    memory.put(BOAR_LAYER + 0xC, &2_i32.to_le_bytes());
    memory.put(BOAR_LAYER + 0x10, &BOAR_SLOTS.to_le_bytes());
    memory.put(BOAR_SLOTS, &[0_u8; 0x48]);
    memory.put(BOAR_SLOTS, &WALK_MOTION.to_le_bytes());
    memory.put(BOAR_SLOTS + 0x4, &frame.to_le_bytes());
    memory.put(BOAR_SLOTS + 0xC, &40_i32.to_le_bytes());
    memory.put(BOAR_SLOTS + 0x10, &[1]);
    memory.put(BOAR_SLOTS + 0x14, &1.0_f32.to_le_bytes());
}

fn riding() -> FakeMemory {
    let mut memory = FakeMemory::new();
    memory.put(
        PLAYER + PLAYER_ACTION_STATE_OFFSET,
        &RIDE_STATE.to_le_bytes(),
    );
    memory.put(
        RIDE_STATE,
        &(FAKE_MODULE_BASE + u64::from(RIDE_BOAR_STATE_VTABLE_RVA)).to_le_bytes(),
    );
    put_handle(
        &mut memory,
        RIDE_STATE + RIDE_BOAR_STATE_BOAR_HANDLE_OFFSET,
        BOAR_POOL_SLOT,
        BOAR_GENERATION,
    );
    put_pool_slot(&mut memory, BOAR_POOL_SLOT, BOAR_GENERATION, BOAR);
    put_handle(
        &mut memory,
        BOAR + ACTOR_SELF_HANDLE_OFFSET,
        BOAR_POOL_SLOT,
        BOAR_GENERATION,
    );
    memory.put(
        BOAR + ACTOR_CLASS_ID_OFFSET,
        &RIDDEN_BOAR_CLASS_ID.to_le_bytes(),
    );
    memory.put(BOAR + ACTOR_NEW_MOTION_PLAYER_FLAG_OFFSET, &[0]);
    memory.put(
        BOAR + ACTOR_OLD_MOTION_LAYER_COUNT_OFFSET,
        &0_i32.to_le_bytes(),
    );
    put_pose(&mut memory, PLAYER, pose(10.0, 3.0, 20.0, 0.0));
    put_pose(&mut memory, BOAR, pose(10.0, 1.5, 20.0, 0.0));
    memory
}

fn close(left: f32, right: f32) -> bool {
    (left - right).abs() < 1e-4
}

#[test]
fn a_player_in_the_ride_state_with_a_live_boar_behind_its_handle_is_riding() {
    assert_eq!(read_ridden_boar_actor(&riding(), PLAYER), Ok(Some(BOAR)));
}

#[test]
fn a_player_with_no_action_state_or_another_one_rides_nothing() {
    let mut idle = riding();
    idle.put(PLAYER + PLAYER_ACTION_STATE_OFFSET, &0_u64.to_le_bytes());
    assert_eq!(read_ridden_boar_actor(&idle, PLAYER), Ok(None));
    let mut climbing = riding();
    climbing.put(
        RIDE_STATE,
        &(FAKE_MODULE_BASE + u64::from(SOME_OTHER_STATE_VTABLE_RVA)).to_le_bytes(),
    );
    assert_eq!(read_ridden_boar_actor(&climbing, PLAYER), Ok(None));
    assert_eq!(read_ridden_boar(&climbing, PLAYER), Ok(None));
}

#[test]
fn a_handle_whose_slot_was_given_away_names_no_boar() {
    let mut reused = riding();
    put_pool_slot(&mut reused, BOAR_POOL_SLOT, BOAR_GENERATION + 1, BOAR);
    assert_eq!(read_ridden_boar_actor(&reused, PLAYER), Ok(None));
    let mut emptied = riding();
    put_pool_slot(&mut emptied, BOAR_POOL_SLOT, BOAR_GENERATION, 0);
    assert_eq!(read_ridden_boar_actor(&emptied, PLAYER), Ok(None));
    let mut outside = riding();
    put_handle(
        &mut outside,
        RIDE_STATE + RIDE_BOAR_STATE_BOAR_HANDLE_OFFSET,
        HANDLE_POOL_SLOT_COUNT,
        BOAR_GENERATION,
    );
    assert_eq!(read_ridden_boar_actor(&outside, PLAYER), Ok(None));
}

#[test]
fn an_actor_of_another_class_behind_the_handle_is_not_a_ridden_boar() {
    let mut prop = riding();
    prop.put(
        BOAR + ACTOR_CLASS_ID_OFFSET,
        &PUPPET_FALLBACK_CLASS_ID.to_le_bytes(),
    );
    assert_eq!(read_ridden_boar_actor(&prop, PLAYER), Ok(None));
}

#[test]
fn an_unreadable_ride_state_is_an_error_and_never_a_boar() {
    let mut memory = FakeMemory::new();
    memory.put(
        PLAYER + PLAYER_ACTION_STATE_OFFSET,
        &RIDE_STATE.to_le_bytes(),
    );
    assert!(read_ridden_boar_actor(&memory, PLAYER).is_err());
    assert!(read_ridden_boar(&memory, PLAYER).is_err());
}

#[test]
fn the_boar_is_sent_as_seen_from_its_rider() {
    let boar = read_ridden_boar(&riding(), PLAYER)
        .expect("readable")
        .expect("riding");
    assert!(close(boar.offset_from_rider[0], 0.0));
    assert!(close(boar.offset_from_rider[1], -1.5));
    assert!(close(boar.offset_from_rider[2], 0.0));
    assert!(close(boar.yaw_from_rider_radians, 0.0));
    assert_eq!(boar.motion, None);
}

#[test]
fn the_offset_is_turned_into_the_riders_frame() {
    let rider = pose(0.0, 0.0, 0.0, FRAC_PI_2);
    let ahead_of_rider = boar_pose_under_rider(
        rider,
        &RiddenBoar {
            offset_from_rider: [0.0, 0.0, 2.0],
            yaw_from_rider_radians: 0.0,
            motion: None,
        },
    );
    let (offset, yaw) = boar_seen_from_rider(rider, ahead_of_rider);
    assert!(close(offset[0], 0.0) && close(offset[2], 2.0) && close(yaw, 0.0));
    assert!(!close(ahead_of_rider.position.x, 0.0));
    assert!(close(ahead_of_rider.position.z, 0.0));
}

#[test]
fn the_copy_puts_the_boar_back_exactly_where_it_stood_under_the_rider() {
    let rider = pose(12.0, 4.0, -7.0, 2.5);
    let boar = pose(12.4, 2.6, -7.9, -3.0);
    let (offset_from_rider, yaw_from_rider_radians) = boar_seen_from_rider(rider, boar);
    let shown = boar_pose_under_rider(
        rider,
        &RiddenBoar {
            offset_from_rider,
            yaw_from_rider_radians,
            motion: None,
        },
    );
    assert!(close(shown.position.x, boar.position.x));
    assert!(close(shown.position.y, boar.position.y));
    assert!(close(shown.position.z, boar.position.z));
    assert!(close(shown.yaw, boar.yaw));
}

#[test]
fn the_boar_keeps_its_place_under_a_rider_shown_somewhere_else() {
    let (offset_from_rider, yaw_from_rider_radians) =
        boar_seen_from_rider(pose(0.0, 0.0, 0.0, 0.0), pose(0.0, -1.5, -0.5, 0.0));
    let boar = RiddenBoar {
        offset_from_rider,
        yaw_from_rider_radians,
        motion: None,
    };
    let shown = boar_pose_under_rider(pose(100.0, 50.0, 100.0, std::f32::consts::PI), &boar);
    assert!(close(shown.position.x, 100.0));
    assert!(close(shown.position.y, 48.5));
    assert!(close(shown.position.z, 100.5));
    assert!(close(shown.yaw.abs(), std::f32::consts::PI));
}

#[test]
fn the_boars_own_body_motion_is_sent_by_its_index_and_frame() {
    let mut memory = riding();
    put_walking_body_layer(&mut memory, 12.5);
    let motion = read_ridden_boar(&memory, PLAYER)
        .expect("readable")
        .expect("riding")
        .motion
        .expect("the boar walks");
    assert_eq!(i32::from(motion.index), WALK_MOTION);
    assert!(close(motion.frame, 12.5));
    assert!(close(motion.speed, 1.0));
    assert!(motion.looping);
    assert_eq!(motion.name_hash, NO_MOTION_NAME_HASH);
}

#[test]
fn a_boar_motion_that_is_not_a_number_is_left_out_and_the_boar_is_still_sent() {
    let mut memory = riding();
    put_walking_body_layer(&mut memory, f32::NAN);
    let boar = read_ridden_boar(&memory, PLAYER)
        .expect("readable")
        .expect("riding");
    assert_eq!(boar.motion, None);
    assert_eq!(boar.validate(), Ok(()));
}

#[test]
fn a_boar_is_made_only_after_its_model_stayed_loaded_for_the_settle_time() {
    let mut settle = BoarPackSettle::default();
    assert!(!settle.settled(false, false, 0));
    assert!(!settle.settled(true, false, 1000));
    assert!(!settle.settled(true, false, 1000 + RIDDEN_BOAR_PACK_SETTLE_MS - 1));
    assert!(settle.settled(true, false, 1000 + RIDDEN_BOAR_PACK_SETTLE_MS));
}

#[test]
fn a_model_that_goes_away_or_a_loading_screen_starts_the_settle_time_again() {
    let mut settle = BoarPackSettle::default();
    assert!(!settle.settled(true, false, 0));
    assert!(settle.settled(true, false, RIDDEN_BOAR_PACK_SETTLE_MS));
    assert!(!settle.settled(false, false, RIDDEN_BOAR_PACK_SETTLE_MS + 1));
    assert!(!settle.settled(true, false, RIDDEN_BOAR_PACK_SETTLE_MS + 2));
    assert!(!settle.settled(true, true, 3 * RIDDEN_BOAR_PACK_SETTLE_MS));
    assert!(!settle.settled(true, false, 3 * RIDDEN_BOAR_PACK_SETTLE_MS + 1));
    assert!(settle.settled(true, false, 4 * RIDDEN_BOAR_PACK_SETTLE_MS + 1));
}

#[test]
fn the_boar_appears_when_the_partner_rides_and_its_model_has_settled() {
    assert_eq!(boar_step(true, false, true, false), BoarStep::Make);
    assert_eq!(boar_step(true, true, true, false), BoarStep::Follow);
    assert_eq!(boar_step(true, true, false, false), BoarStep::Follow);
}

#[test]
fn the_boar_goes_when_the_partner_no_longer_rides() {
    assert_eq!(boar_step(false, true, true, false), BoarStep::Remove);
    assert_eq!(boar_step(false, false, true, false), BoarStep::Nothing);
}

#[test]
fn no_boar_is_made_while_its_model_is_not_settled_or_the_game_closes() {
    assert_eq!(boar_step(true, false, false, false), BoarStep::Wait);
    assert_eq!(boar_step(true, false, true, true), BoarStep::Wait);
    assert_eq!(boar_step(false, true, true, true), BoarStep::Remove);
}

#[test]
fn the_model_is_looked_up_every_tick_while_the_partner_rides_and_now_and_then_otherwise() {
    let mut settle = BoarPackSettle::default();
    assert!(settle.lookup_due(false, 0));
    assert!(!settle.lookup_due(false, 16));
    assert!(!settle.lookup_due(false, RIDDEN_BOAR_PACK_LOOKUP_INTERVAL_MS - 1));
    assert!(settle.lookup_due(false, RIDDEN_BOAR_PACK_LOOKUP_INTERVAL_MS));
    assert!(settle.lookup_due(true, RIDDEN_BOAR_PACK_LOOKUP_INTERVAL_MS + 16));
    assert!(settle.lookup_due(true, RIDDEN_BOAR_PACK_LOOKUP_INTERVAL_MS + 32));
}

#[test]
fn a_partner_who_gets_on_long_after_the_model_loaded_gets_a_boar_at_once() {
    let mut settle = BoarPackSettle::default();
    for now_ms in [0, 1000, 2000, 3000] {
        assert!(settle.lookup_due(false, now_ms));
        settle.settled(true, false, now_ms);
    }
    assert!(settle.lookup_due(true, 3016));
    let settled = settle.settled(true, false, 3016);
    assert_eq!(boar_step(true, false, settled, false), BoarStep::Make);
}
