use rc_game::offsets::PLAYABLE_ACTOR_POINTER_RVA;
use rc_game::{GameError, Position, Replicant};

use crate::fake_memory::FakeMemory;

const ACTOR_ADDRESS: u64 = 0x2_0000_0000;

fn memory_with_actor_pointer(pointer: u64) -> FakeMemory {
    let mut memory = FakeMemory::new();
    memory.put_at_rva(PLAYABLE_ACTOR_POINTER_RVA, &pointer.to_le_bytes());
    memory.put(ACTOR_ADDRESS, &[0_u8; 0xC0]);
    memory
}

#[test]
fn position_follows_the_actor_pointer() {
    let mut game = Replicant::new(memory_with_actor_pointer(ACTOR_ADDRESS));
    let target = Position {
        x: 1.5,
        y: -2.0,
        z: 30.25,
    };
    game.set_position(target).expect("actor memory is writable");
    assert_eq!(game.position(), Ok(target));
    assert_eq!(
        game.memory().get(ACTOR_ADDRESS + 0x9C, 4),
        1.5_f32.to_le_bytes()
    );
    assert_eq!(
        game.memory().get(ACTOR_ADDRESS + 0xAC, 4),
        (-2.0_f32).to_le_bytes()
    );
    assert_eq!(
        game.memory().get(ACTOR_ADDRESS + 0xBC, 4),
        30.25_f32.to_le_bytes()
    );
}

#[test]
fn null_actor_pointer_means_not_spawned() {
    let mut game = Replicant::new(memory_with_actor_pointer(0));
    assert_eq!(game.position(), Err(GameError::PlayableActorNotSpawned));
    assert_eq!(
        game.set_position(Position::default()),
        Err(GameError::PlayableActorNotSpawned)
    );
}
