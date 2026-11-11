use replicant_game::effects::{
    EffectSource, POWER_ACTOR_CLASSES, classify_effect_source, local_book_actor, local_has_book,
    projectile_kind, projectile_owner, read_actor_position,
};
use replicant_game::offsets::{
    ACTOR_TRANSFORM_BLOCK_OFFSET, BOOK_ACTOR_POINTER_RVA, HANDLE_POOL_GENERATIONS_RVA,
    HANDLE_POOL_POINTERS_RVA, HAVE_WHITE_BOOK_FLAG_RVA, NORMAL_BULLET_OWNER_HANDLE_OFFSET,
    NORMAL_BULLET_VTABLE_RVA,
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
        memory.put_at_rva(HANDLE_POOL_POINTERS_RVA + slot * 8, &actor.to_le_bytes());
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
fn a_projectiles_owner_resolves_to_its_actor_and_other_actors_have_none() {
    let mut memory = world();
    bullet_owned_by(&mut memory, ENEMY_SLOT, GENERATION);
    assert_eq!(projectile_owner(&memory, BULLET), Some(ENEMY));
    bullet_owned_by(&mut memory, ENEMY_SLOT, GENERATION + 1);
    assert_eq!(projectile_owner(&memory, BULLET), None);
    assert_eq!(projectile_owner(&memory, BOOK), None);
}

#[test]
fn power_and_bullet_sounds_of_the_player_go_to_the_book_and_others_stay_home() {
    use replicant_game::effects::SoundRoute;
    assert_eq!(EffectSource::Player.sound_route(), Some(SoundRoute::Puppet));
    for source in [
        EffectSource::Book,
        EffectSource::PowerOfPlayer,
        EffectSource::BulletOfPlayer,
    ] {
        assert_eq!(source.sound_route(), Some(SoundRoute::Book));
    }
    for source in [
        EffectSource::PowerOfOther,
        EffectSource::BulletOfOther,
        EffectSource::Other,
    ] {
        assert_eq!(source.sound_route(), None);
    }
}

#[test]
fn the_local_book_is_found_by_the_game_pointer_and_absent_when_it_is_null() {
    let mut memory = world();
    assert_eq!(local_book_actor(&memory), Ok(Some(BOOK)));
    memory.put_at_rva(BOOK_ACTOR_POINTER_RVA, &0_u64.to_le_bytes());
    assert_eq!(local_book_actor(&memory), Ok(None));
}

#[test]
fn the_local_player_has_a_book_only_while_the_games_have_book_flag_is_set() {
    let mut memory = world();
    memory.put_at_rva(HAVE_WHITE_BOOK_FLAG_RVA, &[0]);
    assert!(local_book_actor(&memory).unwrap().is_some());
    assert!(!local_has_book(&memory).unwrap());
    memory.put_at_rva(HAVE_WHITE_BOOK_FLAG_RVA, &[1]);
    assert!(local_has_book(&memory).unwrap());
}

fn spell_owned_by(memory: &mut FakeMemory, class_name: &str, slot: u32) {
    let class = POWER_ACTOR_CLASSES
        .iter()
        .find(|class| class.name == class_name)
        .unwrap();
    let vtable = FAKE_MODULE_BASE + u64::from(class.vtable_rva);
    memory.put(BULLET, &vtable.to_le_bytes());
    let mut handle = slot.to_le_bytes().to_vec();
    handle.extend_from_slice(&GENERATION.to_le_bytes());
    memory.put(BULLET + class.owner_handle_offset, &handle);
}

#[test]
fn a_lance_a_great_lance_and_a_round_blade_of_the_player_are_owned_by_the_player() {
    for name in ["lance", "g lance", "round blade"] {
        let mut memory = world();
        spell_owned_by(&mut memory, name, PLAYER_SLOT);
        assert_eq!(projectile_owner(&memory, BULLET), Some(PLAYER), "{name}");
        assert_eq!(projectile_kind(&memory, BULLET), Some(name));
    }
}

#[test]
fn a_spell_of_another_actor_is_owned_by_that_actor() {
    for name in ["lance", "g lance", "round blade"] {
        let mut memory = world();
        spell_owned_by(&mut memory, name, ENEMY_SLOT);
        assert_eq!(projectile_owner(&memory, BULLET), Some(ENEMY), "{name}");
    }
}

#[test]
fn an_actor_of_an_unknown_class_has_no_projectile_owner_or_kind() {
    let mut memory = world();
    memory.put(BULLET, &OTHER_VTABLE.to_le_bytes());
    assert_eq!(projectile_owner(&memory, BULLET), None);
    assert_eq!(projectile_kind(&memory, BULLET), None);
}

#[test]
fn a_normal_bullet_has_the_bullet_kind() {
    let mut memory = world();
    bullet_owned_by(&mut memory, PLAYER_SLOT, GENERATION);
    assert_eq!(projectile_kind(&memory, BULLET), Some("bullet"));
}
