use rc_game::effects::{
    EffectSource, bullet_owner, classify_effect_source, read_actor_position,
};
use rc_game::offsets::{
    ACTOR_TRANSFORM_BLOCK_OFFSET, BOOK_ACTOR_POINTER_RVA, HANDLE_POOL_GENERATIONS_RVA,
    HANDLE_POOL_POINTERS_RVA, NORMAL_BULLET_OWNER_HANDLE_OFFSET, NORMAL_BULLET_VTABLE_RVA,
};

use crate::fake_memory::{FAKE_MODULE_BASE, FakeMemory};

const PLAYER: u64 = 0x2_0000_0000;
const BOOK: u64 = 0x2_1000_0000;
const BULLET: u64 = 0x2_2000_0000;
const ENEMY: u64 = 0x2_3000_0000;
const OTHER_VTABLE: u64 = 0x1_4999_0000;
const PLAYER_SLOT: u32 = 5;
const ENEMY_SLOT: u32 = 6;
const GENERATION: u32 = 9;

fn world() -> FakeMemory {
    let mut memory = FakeMemory::new();
    memory.put_at_rva(BOOK_ACTOR_POINTER_RVA, &BOOK.to_le_bytes());
    for (slot, actor) in [(PLAYER_SLOT, PLAYER), (ENEMY_SLOT, ENEMY)] {
        memory.put_at_rva(
            HANDLE_POOL_POINTERS_RVA + slot * 8,
            &actor.to_le_bytes(),
        );
        memory.put_at_rva(
            HANDLE_POOL_GENERATIONS_RVA + slot * 4,
            &GENERATION.to_le_bytes(),
        );
    }
    memory.put(BOOK, &OTHER_VTABLE.to_le_bytes());
    memory
}

fn bullet_owned_by(memory: &mut FakeMemory, slot: u32, generation: u32) {
    let vtable = FAKE_MODULE_BASE + u64::from(NORMAL_BULLET_VTABLE_RVA);
    memory.put(BULLET, &vtable.to_le_bytes());
    let mut handle = slot.to_le_bytes().to_vec();
    handle.extend_from_slice(&generation.to_le_bytes());
    memory.put(BULLET + NORMAL_BULLET_OWNER_HANDLE_OFFSET, &handle);
}

#[test]
fn the_player_and_the_book_are_told_apart() {
    let memory = world();
    assert_eq!(
        classify_effect_source(&memory, PLAYER, PLAYER),
        Ok(EffectSource::Player)
    );
    assert_eq!(
        classify_effect_source(&memory, BOOK, PLAYER),
        Ok(EffectSource::Book)
    );
}

#[test]
fn a_bullet_the_player_fired_is_the_players() {
    let mut memory = world();
    bullet_owned_by(&mut memory, PLAYER_SLOT, GENERATION);
    assert_eq!(
        classify_effect_source(&memory, BULLET, PLAYER),
        Ok(EffectSource::BulletOfPlayer)
    );
}

#[test]
fn a_bullet_of_someone_else_or_a_reused_slot_is_not_the_players() {
    let mut memory = world();
    bullet_owned_by(&mut memory, ENEMY_SLOT, GENERATION);
    assert_eq!(
        classify_effect_source(&memory, BULLET, PLAYER),
        Ok(EffectSource::BulletOfOther)
    );
    bullet_owned_by(&mut memory, PLAYER_SLOT, GENERATION + 1);
    assert_eq!(
        classify_effect_source(&memory, BULLET, PLAYER),
        Ok(EffectSource::BulletOfOther)
    );
}

#[test]
fn an_actor_with_another_class_is_other() {
    let mut memory = world();
    memory.put(ENEMY, &OTHER_VTABLE.to_le_bytes());
    assert_eq!(
        classify_effect_source(&memory, ENEMY, PLAYER),
        Ok(EffectSource::Other)
    );
    assert_eq!(
        classify_effect_source(&memory, 0, PLAYER),
        Ok(EffectSource::Other)
    );
}

#[test]
fn an_actor_position_is_the_translation_column_of_its_transform() {
    let mut memory = FakeMemory::new();
    let block = BULLET + ACTOR_TRANSFORM_BLOCK_OFFSET;
    for (row, value) in [1.5_f32, -2.0, 30.25].into_iter().enumerate() {
        memory.put(block + row as u64 * 16 + 12, &value.to_le_bytes());
    }
    assert_eq!(read_actor_position(&memory, BULLET), Ok([1.5, -2.0, 30.25]));
}

#[test]
fn a_bullets_owner_resolves_to_its_actor_and_other_actors_have_none() {
    let mut memory = world();
    bullet_owned_by(&mut memory, ENEMY_SLOT, GENERATION);
    assert_eq!(bullet_owner(&memory, BULLET), Some(ENEMY));
    bullet_owned_by(&mut memory, ENEMY_SLOT, GENERATION + 1);
    assert_eq!(bullet_owner(&memory, BULLET), None);
    assert_eq!(bullet_owner(&memory, BOOK), None);
}
