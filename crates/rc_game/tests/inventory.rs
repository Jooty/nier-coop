use rc_game::offsets::{ENTITY_INVENTORY_OFFSET, ENTITY_RVA, INVENTORY_ITEMS};
use rc_game::{GameError, Replicant};

use crate::fake_memory::FakeMemory;

const MEDICINAL_HERB_RVA: u32 = 0x437_4AE0;

fn memory_with_inventory_block() -> FakeMemory {
    let mut memory = FakeMemory::new();
    memory.put_at_rva(ENTITY_RVA + ENTITY_INVENTORY_OFFSET, &[0_u8; 0x200]);
    memory
}

#[test]
fn set_item_count_writes_one_byte_at_the_item_rva() {
    let mut game = Replicant::new(memory_with_inventory_block());
    game.set_item_count("Medicinal Herb", 5)
        .expect("inventory is writable");
    assert_eq!(game.memory().get_at_rva(MEDICINAL_HERB_RVA, 2), [5, 0]);
    assert_eq!(game.item_count("Medicinal Herb"), Ok(5));
}

#[test]
fn remove_item_zeroes_the_count() {
    let mut game = Replicant::new(memory_with_inventory_block());
    game.set_item_count("Apples", 3)
        .expect("inventory is writable");
    game.remove_item("Apples").expect("inventory is writable");
    assert_eq!(game.item_count("Apples"), Ok(0));
}

#[test]
fn unknown_item_is_refused() {
    let mut game = Replicant::new(memory_with_inventory_block());
    let result = game.set_item_count("Moon Cheese", 1);
    assert_eq!(
        result,
        Err(GameError::UnknownItem("Moon Cheese".to_owned()))
    );
}

#[test]
fn every_item_sits_inside_the_inventory_block_once() {
    let first = ENTITY_RVA + ENTITY_INVENTORY_OFFSET;
    let mut seen = std::collections::BTreeSet::new();
    for (name, rva) in INVENTORY_ITEMS {
        assert!(rva >= first, "{name} sits before the inventory block");
        assert!(seen.insert(rva), "{name} shares its rva with another item");
    }
    assert_eq!(INVENTORY_ITEMS.len(), 252);
}

#[test]
fn every_item_name_is_unique() {
    let names: std::collections::BTreeSet<_> =
        INVENTORY_ITEMS.iter().map(|(name, _)| *name).collect();
    assert_eq!(names.len(), INVENTORY_ITEMS.len());
}
