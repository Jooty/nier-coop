use rc_game::GameMemory;
use rc_game::boss_messages::{
    LIFE_CALLBACK_KIND, LifeCallbackEntry, REGISTRY_ENTRIES_OFFSET, REGISTRY_ENTRY_COUNT,
    REGISTRY_ENTRY_DATA_OFFSET, REGISTRY_ENTRY_DATA_WORDS_SCANNED, REGISTRY_ENTRY_KIND_OFFSET,
    REGISTRY_ENTRY_STRIDE_BYTES, life_callback_targets_group,
};
use rc_game::cutscene_skip::TRIGGER_MANAGER_RVA;

use crate::build_gate::VerifiedBuild;
use crate::memory::InProcessMemory;

pub fn group_has_registered_life_callback(build: &VerifiedBuild, group: u16) -> bool {
    let memory = InProcessMemory::new(build);
    let registry = build.module_base() + u64::from(TRIGGER_MANAGER_RVA);
    let mut scanned: Vec<[i32; REGISTRY_ENTRY_DATA_WORDS_SCANNED]> = Vec::new();
    for index in 0..REGISTRY_ENTRY_COUNT {
        let entry = registry + REGISTRY_ENTRIES_OFFSET + index * REGISTRY_ENTRY_STRIDE_BYTES;
        let Ok(kind) = memory.read_i32(entry + REGISTRY_ENTRY_KIND_OFFSET) else {
            return false;
        };
        if kind != LIFE_CALLBACK_KIND {
            continue;
        }
        let mut words = [0_i32; REGISTRY_ENTRY_DATA_WORDS_SCANNED];
        for (word_index, word) in words.iter_mut().enumerate() {
            let address = entry + REGISTRY_ENTRY_DATA_OFFSET + (word_index as u64) * 4;
            let Ok(value) = memory.read_i32(address) else {
                return false;
            };
            *word = value;
        }
        scanned.push(words);
    }
    let entries: Vec<LifeCallbackEntry<'_>> = scanned
        .iter()
        .map(|words| LifeCallbackEntry {
            kind: LIFE_CALLBACK_KIND,
            data_words: words,
        })
        .collect();
    life_callback_targets_group(&entries, i32::from(group))
}
