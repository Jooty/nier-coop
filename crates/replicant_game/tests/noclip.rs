use replicant_game::offsets::{
    ACTOR_DISPLACEMENT_PENDING_FLAG_OFFSET, ACTOR_FALL_DISTANCE_OFFSET,
    ACTOR_PENDING_DISPLACEMENT_OFFSET, ACTOR_VERTICAL_SPEED_OFFSET, NOCLIP_FAST_MULTIPLIER,
    NOCLIP_UNITS_PER_STEP, PLAYABLE_ACTOR_POINTER_RVA,
};
use replicant_game::{Displacement, GameError, NoclipInput, Replicant};

use crate::fake_memory::FakeMemory;

const ACTOR_ADDRESS: u64 = 0x2_0000_0000;
const ACTOR_BYTES: usize = 0x1500;

fn memory_with_actor_pointer(pointer: u64) -> FakeMemory {
    let mut memory = FakeMemory::new();
    memory.put_at_rva(PLAYABLE_ACTOR_POINTER_RVA, &pointer.to_le_bytes());
    memory.put(ACTOR_ADDRESS, &[0xAB_u8; ACTOR_BYTES]);
    memory
}

#[test]
fn no_keys_means_hovering_in_place() {
    assert_eq!(
        NoclipInput::default().displacement(),
        Displacement::default()
    );
}

#[test]
fn up_and_down_move_along_y_only() {
    let up = NoclipInput {
        vertical: 1,
        ..NoclipInput::default()
    };
    let down = NoclipInput {
        vertical: -1,
        ..NoclipInput::default()
    };
    assert_eq!(up.displacement().y, NOCLIP_UNITS_PER_STEP);
    assert_eq!(down.displacement().y, -NOCLIP_UNITS_PER_STEP);
    assert_eq!(up.displacement().x, 0.0);
    assert_eq!(up.displacement().z, 0.0);
}

#[test]
fn diagonal_walking_is_no_faster_than_straight_walking() {
    let diagonal = NoclipInput {
        forward: 1,
        strafe: 1,
        ..NoclipInput::default()
    }
    .displacement();
    let length = diagonal.x.hypot(diagonal.z);
    assert!((length - NOCLIP_UNITS_PER_STEP).abs() < 1e-6);
}

#[test]
fn fast_multiplies_the_step() {
    let fast = NoclipInput {
        forward: 1,
        fast: true,
        ..NoclipInput::default()
    };
    assert_eq!(
        fast.displacement().z,
        NOCLIP_UNITS_PER_STEP * NOCLIP_FAST_MULTIPLIER
    );
}

#[test]
fn a_noclip_step_sets_displacement_flag_and_zeroes_falling() {
    let mut game = Replicant::new(memory_with_actor_pointer(ACTOR_ADDRESS));
    let step = Displacement {
        x: 0.5,
        y: -0.25,
        z: 2.0,
    };
    game.write_noclip_step(ACTOR_ADDRESS, step)
        .expect("actor memory is writable");
    let memory = game.memory();
    let base = ACTOR_ADDRESS + ACTOR_PENDING_DISPLACEMENT_OFFSET;
    assert_eq!(memory.get(base, 4), 0.5_f32.to_le_bytes());
    assert_eq!(memory.get(base + 4, 4), (-0.25_f32).to_le_bytes());
    assert_eq!(memory.get(base + 8, 4), 2.0_f32.to_le_bytes());
    assert_eq!(
        memory.get(ACTOR_ADDRESS + ACTOR_DISPLACEMENT_PENDING_FLAG_OFFSET, 1),
        [1]
    );
    assert_eq!(
        memory.get(ACTOR_ADDRESS + ACTOR_VERTICAL_SPEED_OFFSET, 4),
        0.0_f32.to_le_bytes()
    );
    assert_eq!(
        memory.get(ACTOR_ADDRESS + ACTOR_FALL_DISTANCE_OFFSET, 4),
        0.0_f32.to_le_bytes()
    );
}

#[test]
fn the_playable_actor_address_is_the_pointer_in_the_module() {
    let game = Replicant::new(memory_with_actor_pointer(ACTOR_ADDRESS));
    assert_eq!(game.playable_actor(), Ok(ACTOR_ADDRESS));
    let empty = Replicant::new(memory_with_actor_pointer(0));
    assert_eq!(
        empty.playable_actor(),
        Err(GameError::PlayableActorNotSpawned)
    );
}
