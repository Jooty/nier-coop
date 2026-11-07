use rc_game::effects::{EffectSource, POWER_ACTOR_CLASSES, classify_effect_source};
use rc_game::offsets::{
    ACTOR_SELF_HANDLE_OFFSET, ACTOR_TRANSFORM_BLOCK_BYTES, ACTOR_TRANSFORM_BLOCK_OFFSET,
    BLACK_HAND_OWNER_HANDLE_OFFSET, BLACK_HAND_VTABLE_RVA, BOOK_ACTOR_POINTER_RVA,
    HANDLE_POOL_GENERATIONS_RVA, HANDLE_POOL_POINTERS_RVA, HANDLE_POOL_SLOT_COUNT,
};
use rc_game::power_models::{
    MAX_PARTNER_POWER_MODELS, MAX_SIGHTED_HANDS, POWER_MODEL_KINDS, POWER_MODEL_STALE_MS,
    PowerModelTable, hands_of_player, power_model_pack_name,
};

use crate::fake_memory::{FAKE_MODULE_BASE, FakeMemory};

const PLAYER: u64 = 0x2_0000_0000;
const ENEMY: u64 = 0x2_3000_0000;
const HAND_HEAP: u64 = 0x2_4000_0000;
const HAND_STRIDE: u64 = 0x2_0000;
const PLAYER_SLOT: u32 = 5;
const ENEMY_SLOT: u32 = 6;
const FIRST_HAND_SLOT: u32 = 100;
const GENERATION: u32 = 9;

fn pool_entry(memory: &mut FakeMemory, slot: u32, actor: u64, generation: u32) {
    memory.put_at_rva(HANDLE_POOL_POINTERS_RVA + slot * 8, &actor.to_le_bytes());
    memory.put_at_rva(
        HANDLE_POOL_GENERATIONS_RVA + slot * 4,
        &generation.to_le_bytes(),
    );
}

fn handle_bytes(slot: u32, generation: u32) -> Vec<u8> {
    let mut bytes = slot.to_le_bytes().to_vec();
    bytes.extend_from_slice(&generation.to_le_bytes());
    bytes
}

fn world() -> FakeMemory {
    let mut memory = FakeMemory::new();
    let empty_table = vec![0_u8; HANDLE_POOL_SLOT_COUNT as usize * 8];
    memory.put_at_rva(HANDLE_POOL_POINTERS_RVA, &empty_table);
    memory.put_at_rva(BOOK_ACTOR_POINTER_RVA, &0_u64.to_le_bytes());
    pool_entry(&mut memory, PLAYER_SLOT, PLAYER, GENERATION);
    pool_entry(&mut memory, ENEMY_SLOT, ENEMY, GENERATION);
    memory
}

fn hand(memory: &mut FakeMemory, index: u32, owner_slot: u32, owner_generation: u32) -> u64 {
    let actor = HAND_HEAP + u64::from(index) * HAND_STRIDE;
    let vtable = FAKE_MODULE_BASE + u64::from(BLACK_HAND_VTABLE_RVA);
    memory.put(actor, &vtable.to_le_bytes());
    memory.put(
        actor + BLACK_HAND_OWNER_HANDLE_OFFSET,
        &handle_bytes(owner_slot, owner_generation),
    );
    let slot = FIRST_HAND_SLOT + index;
    memory.put(
        actor + ACTOR_SELF_HANDLE_OFFSET,
        &handle_bytes(slot, GENERATION),
    );
    pool_entry(memory, slot, actor, GENERATION);
    memory.put(
        actor + ACTOR_TRANSFORM_BLOCK_OFFSET,
        &[0_u8; ACTOR_TRANSFORM_BLOCK_BYTES],
    );
    for (row, value) in [4.0_f32, 5.0, 6.0].into_iter().enumerate() {
        memory.put(
            actor + ACTOR_TRANSFORM_BLOCK_OFFSET + row as u64 * 16 + 12,
            &value.to_le_bytes(),
        );
    }
    actor
}

#[test]
fn every_power_class_of_the_player_is_the_players_and_of_an_enemy_is_not() {
    for class in POWER_ACTOR_CLASSES {
        let mut memory = world();
        let actor = HAND_HEAP;
        let vtable = FAKE_MODULE_BASE + u64::from(class.vtable_rva);
        memory.put(actor, &vtable.to_le_bytes());
        memory.put(
            actor + class.owner_handle_offset,
            &handle_bytes(PLAYER_SLOT, GENERATION),
        );
        assert_eq!(
            classify_effect_source(&memory, actor, PLAYER),
            Ok(EffectSource::PowerOfPlayer),
            "{}",
            class.name
        );
        memory.put(
            actor + class.owner_handle_offset,
            &handle_bytes(ENEMY_SLOT, GENERATION),
        );
        assert_eq!(
            classify_effect_source(&memory, actor, PLAYER),
            Ok(EffectSource::PowerOfOther),
            "{}",
            class.name
        );
    }
}

#[test]
fn only_the_players_live_hands_are_sighted_with_their_pose() {
    let mut memory = world();
    let mine = hand(&mut memory, 0, PLAYER_SLOT, GENERATION);
    hand(&mut memory, 1, ENEMY_SLOT, GENERATION);
    hand(&mut memory, 2, PLAYER_SLOT, GENERATION + 1);
    let sighted = hands_of_player(&memory, PLAYER).unwrap();
    assert_eq!(sighted.len(), 1);
    assert_eq!(sighted[0].actor, mine);
    assert_eq!(sighted[0].slot, FIRST_HAND_SLOT as u16);
    assert_eq!(sighted[0].pose.position.x, 4.0);
    assert_eq!(sighted[0].pose.position.z, 6.0);
}

#[test]
fn a_hand_whose_slot_was_reused_is_not_sighted() {
    let mut memory = world();
    let stale = hand(&mut memory, 0, PLAYER_SLOT, GENERATION);
    pool_entry(&mut memory, FIRST_HAND_SLOT, stale, GENERATION + 1);
    assert!(hands_of_player(&memory, PLAYER).unwrap().is_empty());
}

#[test]
fn no_more_than_the_limit_of_hands_are_sighted() {
    let mut memory = world();
    for index in 0..8 {
        hand(&mut memory, index, PLAYER_SLOT, GENERATION);
    }
    assert_eq!(
        hands_of_player(&memory, PLAYER).unwrap().len(),
        MAX_SIGHTED_HANDS
    );
}

#[test]
fn a_partner_hand_that_goes_quiet_is_dropped_and_the_table_is_capped() {
    let mut table = PowerModelTable::default();
    assert!(table.note(1, 0));
    assert!(table.note(2, 100));
    assert!(table.note(1, 700));
    assert_eq!(table.take_stale(POWER_MODEL_STALE_MS + 100), vec![2]);
    assert_eq!(table.len(), 1);
    for instance in 10..10 + MAX_PARTNER_POWER_MODELS as u16 {
        table.note(instance, 700);
    }
    assert!(!table.note(99, 700));
    assert!(table.note(1, 720));
    assert_eq!(table.len(), MAX_PARTNER_POWER_MODELS);
    assert_eq!(table.take_all().len(), MAX_PARTNER_POWER_MODELS);
    assert!(table.is_empty());
}

#[test]
fn every_hand_kind_has_its_own_pack_name() {
    let mut names: Vec<&str> = POWER_MODEL_KINDS
        .iter()
        .map(|kind| power_model_pack_name(*kind))
        .collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), POWER_MODEL_KINDS.len());
}
