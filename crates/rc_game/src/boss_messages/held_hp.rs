pub fn held_hp_to_restore(before: i32, after: i32) -> Option<i32> {
    (before > 0 && after > 0 && after < before).then_some(before)
}
