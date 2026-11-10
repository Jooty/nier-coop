use replicant_game::offsets::{
    ENTITY_GOLD_OFFSET, ENTITY_HEALTH_OFFSET, ENTITY_NAME_OFFSET, ENTITY_RVA,
};
use replicant_game::{GameError, MemoryError, Replicant};

use crate::fake_memory::{FAKE_MODULE_BASE, FakeMemory};

fn entity_with_zeroed_block() -> FakeMemory {
    let mut memory = FakeMemory::new();
    memory.put_at_rva(ENTITY_RVA, &[0_u8; 0x4A8]);
    memory
}

#[test]
fn gold_is_read_from_the_entity_gold_field() {
    let mut memory = entity_with_zeroed_block();
    memory.put_at_rva(ENTITY_RVA + ENTITY_GOLD_OFFSET, &1234_i32.to_le_bytes());
    assert_eq!(Replicant::new(memory).gold(), Ok(1234));
}

#[test]
fn set_health_writes_the_entity_health_field() {
    let mut game = Replicant::new(entity_with_zeroed_block());
    game.set_health(77).expect("health is writable");
    let written = game
        .memory()
        .get_at_rva(ENTITY_RVA + ENTITY_HEALTH_OFFSET, 4);
    assert_eq!(written, 77_i32.to_le_bytes());
}

#[test]
fn magic_level_and_playtime_round_trip() {
    let mut game = Replicant::new(entity_with_zeroed_block());
    game.set_magic(12.5).expect("magic is writable");
    game.set_level(9).expect("level is writable");
    game.set_playtime(3600.25).expect("playtime is writable");
    assert_eq!(game.magic(), Ok(12.5));
    assert_eq!(game.level(), Ok(9));
    assert_eq!(game.playtime(), Ok(3600.25));
}

#[test]
fn name_stops_at_the_first_nul() {
    let mut memory = entity_with_zeroed_block();
    memory.put_at_rva(ENTITY_RVA + ENTITY_NAME_OFFSET, b"Nier\0junk");
    assert_eq!(Replicant::new(memory).name().as_deref(), Ok("Nier"));
}

#[test]
fn set_zone_round_trips_short_text() {
    let mut game = Replicant::new(entity_with_zeroed_block());
    game.set_zone("Village").expect("zone is writable");
    assert_eq!(game.zone().as_deref(), Ok("Village"));
}

#[test]
fn set_name_refuses_text_longer_than_the_field() {
    let mut game = Replicant::new(entity_with_zeroed_block());
    let result = game.set_name(&"n".repeat(64));
    assert_eq!(
        result,
        Err(GameError::TextTooLong {
            len: 65,
            capacity: 0x20
        })
    );
}

#[test]
fn reading_unmapped_memory_reports_the_address() {
    let game = Replicant::new(FakeMemory::new());
    let address = FAKE_MODULE_BASE + u64::from(ENTITY_RVA + ENTITY_GOLD_OFFSET);
    assert_eq!(
        game.gold(),
        Err(GameError::Memory(MemoryError::Unreadable { address }))
    );
}
