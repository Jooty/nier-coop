use crate::story_calls::{LIBRARY_PACK_HASH, STORY_CALL_TABLE, StoryCallKind};
use crate::story_progress::fnv_hash;

pub fn name_hash(name: &str) -> u64 {
    fnv_hash(name.as_bytes())
}

pub fn kind_in_table(
    table: &[(u64, u64, StoryCallKind)],
    pack_hash: u64,
    function_hash: u64,
) -> Option<StoryCallKind> {
    table
        .binary_search_by_key(&(pack_hash, function_hash), |&(pack, function, _)| {
            (pack, function)
        })
        .ok()
        .map(|index| table[index].2)
}

pub fn story_call_kind(pack_hash: u64, function_hash: u64) -> StoryCallKind {
    kind_in_table(&STORY_CALL_TABLE, pack_hash, function_hash)
        .or_else(|| kind_in_table(&STORY_CALL_TABLE, LIBRARY_PACK_HASH, function_hash))
        .unwrap_or(StoryCallKind::Local)
}

pub fn story_call_kind_by_name(pack: &str, function: &str) -> StoryCallKind {
    story_call_kind(name_hash(pack), name_hash(function))
}
