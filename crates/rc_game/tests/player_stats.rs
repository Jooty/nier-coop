use rc_game::Replicant;
use rc_game::offsets::{
    ENTITY_EXPERIENCE_OFFSET, ENTITY_RVA, PLAYER_PARAM_ARRAY_RVA, PLAYER_PARAM_MAX_HEALTH_OFFSET,
    PLAYER_PARAM_MAX_MAGIC_OFFSET,
};

use crate::fake_memory::FakeMemory;

#[test]
fn max_health_reads_the_first_player_param() {
    let mut memory = FakeMemory::new();
    memory.put_at_rva(
        PLAYER_PARAM_ARRAY_RVA + PLAYER_PARAM_MAX_HEALTH_OFFSET,
        &840_i32.to_le_bytes(),
    );
    assert_eq!(Replicant::new(memory).max_health(), Ok(840));
}

#[test]
fn max_magic_reads_the_first_player_param() {
    let mut memory = FakeMemory::new();
    memory.put_at_rva(
        PLAYER_PARAM_ARRAY_RVA + PLAYER_PARAM_MAX_MAGIC_OFFSET,
        &200.0_f32.to_le_bytes(),
    );
    assert_eq!(Replicant::new(memory).max_magic(), Ok(200.0));
}

#[test]
fn experience_reads_the_entity_experience_field() {
    let mut memory = FakeMemory::new();
    memory.put_at_rva(
        ENTITY_RVA + ENTITY_EXPERIENCE_OFFSET,
        &321_i32.to_le_bytes(),
    );
    assert_eq!(Replicant::new(memory).experience(), Ok(321));
}
