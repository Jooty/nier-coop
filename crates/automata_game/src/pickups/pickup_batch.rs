use crate::pickups::ItemPickup;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GivenItem {
    pub item_id: u32,
    pub key_item: bool,
}

/// A give is a pickup to tell when the game added the item and showed its own line for it.
pub fn is_told_give(show_log: i32, added: i32) -> bool {
    show_log != 0 && added != 0
}

/// One pickup per item, in the order the items were first given, with how many were given.
pub fn merge_given_items(given: &[GivenItem]) -> Vec<ItemPickup> {
    let mut merged: Vec<ItemPickup> = Vec::new();
    for item in given {
        if let Some(pickup) = merged
            .iter_mut()
            .find(|pickup| pickup.item_id == item.item_id)
        {
            pickup.count = pickup.count.saturating_add(1);
            pickup.key_item |= item.key_item;
        } else {
            merged.push(ItemPickup {
                item_id: item.item_id,
                count: 1,
                key_item: item.key_item,
            });
        }
    }
    merged
}
