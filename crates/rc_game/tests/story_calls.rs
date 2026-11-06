use rc_game::story_calls::{
    LIBRARY_PACK_HASH, STORY_CALL_TABLE, StoryCallKind, kind_in_table, name_hash, story_call_kind,
    story_call_kind_by_name,
};

const PACK_A: u64 = 10;
const PACK_B: u64 = 20;
const FUNCTION_ONE: u64 = 1;
const FUNCTION_TWO: u64 = 2;
const FUNCTION_THREE: u64 = 3;

const SMALL_TABLE: [(u64, u64, StoryCallKind); 4] = [
    (PACK_A, FUNCTION_ONE, StoryCallKind::Story),
    (PACK_A, FUNCTION_TWO, StoryCallKind::Container),
    (PACK_B, FUNCTION_ONE, StoryCallKind::Container),
    (PACK_B, FUNCTION_TWO, StoryCallKind::Story),
];

#[test]
fn the_name_hash_is_fnv_1a_64() {
    assert_eq!(name_hash(""), 0xCBF2_9CE4_8422_2325);
    assert_eq!(name_hash("a"), 0xAF63_DC4C_8601_EC8C);
}

#[test]
fn the_library_hash_is_the_hash_of_its_pack_name() {
    assert_eq!(LIBRARY_PACK_HASH, name_hash("__lib__"));
}

#[test]
fn a_lookup_needs_both_the_pack_and_the_function() {
    assert_eq!(
        kind_in_table(&SMALL_TABLE, PACK_A, FUNCTION_ONE),
        Some(StoryCallKind::Story)
    );
    assert_eq!(
        kind_in_table(&SMALL_TABLE, PACK_B, FUNCTION_ONE),
        Some(StoryCallKind::Container)
    );
    assert_eq!(kind_in_table(&SMALL_TABLE, PACK_A, FUNCTION_THREE), None);
    assert_eq!(kind_in_table(&SMALL_TABLE, 30, FUNCTION_ONE), None);
}

#[test]
fn only_story_calls_are_relayed() {
    assert!(StoryCallKind::Story.is_relayed());
    assert!(!StoryCallKind::Container.is_relayed());
    assert!(!StoryCallKind::Local.is_relayed());
}

#[test]
fn the_table_is_sorted_with_unique_keys() {
    let keys: Vec<(u64, u64)> = STORY_CALL_TABLE.iter().map(|&(p, f, _)| (p, f)).collect();
    assert!(keys.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn the_table_holds_no_local_entries() {
    assert!(
        STORY_CALL_TABLE
            .iter()
            .all(|&(_, _, kind)| kind != StoryCallKind::Local)
    );
}

#[test]
fn every_table_entry_is_found_by_the_real_lookup() {
    for &(pack, function, kind) in &STORY_CALL_TABLE {
        assert_eq!(story_call_kind(pack, function), kind);
    }
}

#[test]
fn a_function_in_no_table_row_is_local() {
    assert_eq!(story_call_kind(1, 1), StoryCallKind::Local);
    assert_eq!(
        story_call_kind_by_name("unknown_pack", "unknown_function"),
        StoryCallKind::Local
    );
}

#[test]
fn a_library_row_answers_for_any_pack() {
    let &(_, function, kind) = STORY_CALL_TABLE
        .iter()
        .find(|&&(pack, _, _)| pack == LIBRARY_PACK_HASH)
        .expect("the table has a library row");
    assert_eq!(story_call_kind(0x1234, function), kind);
}
