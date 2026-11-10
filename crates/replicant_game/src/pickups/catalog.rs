use crate::offsets::{
    FIRST_INVENTORY_ITEM_RVA, INVENTORY_ITEMS, KEY_ITEM_FIRST_ID, KEY_ITEM_LAST_ID, STORY_ITEM_IDS,
};

pub fn item_name(item_id: u16) -> Option<&'static str> {
    let rva = FIRST_INVENTORY_ITEM_RVA + u32::from(item_id);
    INVENTORY_ITEMS
        .iter()
        .find(|(_, item_rva)| *item_rva == rva)
        .map(|(name, _)| *name)
}

pub fn is_key_item(item_id: u16) -> bool {
    (KEY_ITEM_FIRST_ID..=KEY_ITEM_LAST_ID).contains(&item_id) || is_story_item(item_id)
}

pub fn is_story_item(item_id: u16) -> bool {
    STORY_ITEM_IDS.contains(&item_id)
}

pub fn story_item_raise_amount(sender_count: u8, own_count: u8) -> u8 {
    sender_count.saturating_sub(own_count)
}
