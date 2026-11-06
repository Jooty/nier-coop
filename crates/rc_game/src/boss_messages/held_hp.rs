use crate::boss_messages::LIFE_CALLBACK_KIND;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LifeCallbackEntry<'words> {
    pub kind: i32,
    pub data_words: &'words [i32],
}

pub fn held_hp_to_restore(before: i32, after: i32) -> Option<i32> {
    (before > 0 && after > 0 && after < before).then_some(before)
}

pub fn life_callback_targets_group(entries: &[LifeCallbackEntry<'_>], group: i32) -> bool {
    entries
        .iter()
        .any(|entry| entry.kind == LIFE_CALLBACK_KIND && entry.data_words.contains(&group))
}
