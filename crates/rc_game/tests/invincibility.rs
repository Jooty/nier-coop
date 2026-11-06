use rc_game::Replicant;
use rc_game::offsets::{
    ENTITY_HEALTH_OFFSET, ENTITY_RVA, LOCAL_PLAYER_SLOT_INDEX, PLAYER_PARAM_SLOT_INDEX_OFFSET,
};

use crate::fake_memory::FakeMemory;

const PLAYER_PARAM_ADDRESS: u64 = 0x2_0000_0000;
const CURRENT_HEALTH: i32 = 500;

fn memory_with(slot: i32, health: i32) -> FakeMemory {
    let mut memory = FakeMemory::new();
    memory.put(
        PLAYER_PARAM_ADDRESS + PLAYER_PARAM_SLOT_INDEX_OFFSET,
        &slot.to_le_bytes(),
    );
    memory.put_at_rva(ENTITY_RVA + ENTITY_HEALTH_OFFSET, &health.to_le_bytes());
    memory
}

fn hurts(slot: i32, health: i32, requested: i32) -> bool {
    Replicant::new(memory_with(slot, health))
        .health_write_hurts_local_player(PLAYER_PARAM_ADDRESS, requested)
        .expect("fake memory holds both values")
}

#[test]
fn lowering_the_local_players_health_hurts() {
    assert!(hurts(
        LOCAL_PLAYER_SLOT_INDEX,
        CURRENT_HEALTH,
        CURRENT_HEALTH - 1
    ));
    assert!(hurts(LOCAL_PLAYER_SLOT_INDEX, CURRENT_HEALTH, 0));
    assert!(hurts(LOCAL_PLAYER_SLOT_INDEX, CURRENT_HEALTH, -40));
}

#[test]
fn healing_and_unchanged_health_do_not_hurt() {
    assert!(!hurts(
        LOCAL_PLAYER_SLOT_INDEX,
        CURRENT_HEALTH,
        CURRENT_HEALTH
    ));
    assert!(!hurts(
        LOCAL_PLAYER_SLOT_INDEX,
        CURRENT_HEALTH,
        CURRENT_HEALTH + 100
    ));
}

#[test]
fn other_party_members_are_not_protected() {
    assert!(!hurts(1, CURRENT_HEALTH, 10));
}

#[test]
fn a_player_already_at_zero_health_is_not_blocked() {
    assert!(!hurts(LOCAL_PLAYER_SLOT_INDEX, 0, 0));
}

#[test]
fn only_a_write_down_to_one_or_less_from_above_one_is_lethal() {
    let lethal = |slot, health, requested| {
        Replicant::new(memory_with(slot, health))
            .health_write_is_lethal_to_local_player(PLAYER_PARAM_ADDRESS, requested)
            .expect("fake memory holds both values")
    };
    assert!(lethal(LOCAL_PLAYER_SLOT_INDEX, CURRENT_HEALTH, 1));
    assert!(lethal(LOCAL_PLAYER_SLOT_INDEX, CURRENT_HEALTH, 0));
    assert!(!lethal(LOCAL_PLAYER_SLOT_INDEX, CURRENT_HEALTH, 2));
    assert!(!lethal(LOCAL_PLAYER_SLOT_INDEX, 1, 0));
    assert!(!lethal(LOCAL_PLAYER_SLOT_INDEX + 1, CURRENT_HEALTH, 0));
}
