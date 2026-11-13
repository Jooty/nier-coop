use crate::pickups::KEY_ITEM_KIND;

pub fn is_key_item_kind(kind: i32) -> bool {
    kind == KEY_ITEM_KIND
}

/// The index of the item's record in the game's item table, sorted by id, as the game's own
/// give-item call searches it; `id_at` reads the id of one record.
pub fn find_item_record(
    record_count: i32,
    item_id: i32,
    id_at: impl Fn(i32) -> Option<i32>,
) -> Option<i32> {
    let mut low = 0;
    let mut high = record_count - 1;
    while low <= high {
        let middle = low + (high - low) / 2;
        let found = id_at(middle)?;
        match found.cmp(&item_id) {
            std::cmp::Ordering::Equal => return Some(middle),
            std::cmp::Ordering::Less => low = middle + 1,
            std::cmp::Ordering::Greater => high = middle - 1,
        }
    }
    None
}
