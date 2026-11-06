use crate::story_calls::{STORY_CALL_TABLE, StoryCallKind, name_hash, story_call_kind};

pub fn pack_hash_of_phase(phase_name: &str) -> u64 {
    name_hash(phase_name)
}

pub fn relay_kind(pack_hash: u64, function_hash: u64) -> Option<StoryCallKind> {
    let first_row = STORY_CALL_TABLE.partition_point(|&(pack, _, _)| pack < pack_hash);
    let pack_has_rows = STORY_CALL_TABLE
        .get(first_row)
        .is_some_and(|&(pack, _, _)| pack == pack_hash);
    pack_has_rows.then(|| story_call_kind(pack_hash, function_hash))
}
