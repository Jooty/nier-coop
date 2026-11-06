use rc_game::offsets::PLAYABLE_ACTOR_POINTER_RVA;
use rc_game::{BodyMotion, PhaseEntry, Replicant};

use crate::fake_memory::FakeMemory;

const ACTOR: u64 = 0x2_0000_0000;
const MOTION_PLAYER: u64 = 0x3_0000_0000;
const LAYER: u64 = 0x3_1000_0000;
const SLOTS: u64 = 0x3_2000_0000;
const SLOT_STRIDE: u64 = 0x258;

fn memory_with_actor() -> FakeMemory {
    let mut memory = FakeMemory::new();
    memory.put_at_rva(PLAYABLE_ACTOR_POINTER_RVA, &ACTOR.to_le_bytes());
    memory.put(ACTOR, &vec![0_u8; 0x16400]);
    memory
}

fn memory_with_motion(current: i32, motion_index: i32, frame: f32) -> FakeMemory {
    let mut memory = memory_with_actor();
    memory.put(ACTOR + 0x163C0 + 0x10, &MOTION_PLAYER.to_le_bytes());
    memory.put(MOTION_PLAYER + 0xA8, &LAYER.to_le_bytes());
    memory.put(LAYER + 0x8, &current.to_le_bytes());
    memory.put(LAYER + 0xC, &3_i32.to_le_bytes());
    memory.put(LAYER + 0x10, &SLOTS.to_le_bytes());
    let slot = SLOTS + SLOT_STRIDE * current.max(0) as u64;
    memory.put(slot, &motion_index.to_le_bytes());
    memory.put(slot + 4, &frame.to_le_bytes());
    memory
}

#[test]
fn facing_along_plus_x_is_a_quarter_turn() {
    let mut memory = memory_with_actor();
    memory.put(ACTOR + 0xB0, &1.0_f32.to_le_bytes());
    memory.put(ACTOR + 0xB8, &0.0_f32.to_le_bytes());
    let yaw = Replicant::new(memory).yaw_radians().expect("actor is set");
    assert!((yaw - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
}

#[test]
fn facing_along_plus_z_is_zero_yaw() {
    let mut memory = memory_with_actor();
    memory.put(ACTOR + 0xB8, &1.0_f32.to_le_bytes());
    let yaw = Replicant::new(memory).yaw_radians().expect("actor is set");
    assert!(yaw.abs() < 1e-6);
}

#[test]
fn body_motion_reads_the_current_slot_of_layer_zero() {
    let game = Replicant::new(memory_with_motion(1, 889, 7.5));
    assert_eq!(
        game.body_motion(),
        Ok(Some(BodyMotion {
            index: 889,
            frame: 7.5
        }))
    );
}

#[test]
fn free_slot_means_no_body_motion() {
    let game = Replicant::new(memory_with_motion(0, -1, 0.0));
    assert_eq!(game.body_motion(), Ok(None));
}

#[test]
fn null_motion_player_means_no_body_motion() {
    let game = Replicant::new(memory_with_actor());
    assert_eq!(game.body_motion(), Ok(None));
}

#[test]
fn slot_index_past_the_slot_count_means_no_body_motion() {
    let mut memory = memory_with_motion(0, 5, 1.0);
    memory.put(LAYER + 0x8, &9_i32.to_le_bytes());
    assert_eq!(Replicant::new(memory).body_motion(), Ok(None));
}

#[test]
fn phase_index_matches_table_position() {
    let first = rc_game::PHASE_TABLE[0].name;
    assert_eq!(PhaseEntry::index_of(first), Some(0));
    assert_eq!(PhaseEntry::index_of("NOT_A_PHASE"), None);
}
