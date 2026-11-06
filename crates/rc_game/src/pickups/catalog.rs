use crate::offsets::{
    FIRST_INVENTORY_ITEM_RVA, INVENTORY_ITEMS, KEY_ITEM_FIRST_ID, KEY_ITEM_LAST_ID,
};

pub fn item_name(item_id: u16) -> Option<&'static str> {
    let rva = FIRST_INVENTORY_ITEM_RVA + u32::from(item_id);
    INVENTORY_ITEMS
        .iter()
        .find(|(_, item_rva)| *item_rva == rva)
        .map(|(name, _)| *name)
}

pub fn is_key_item(item_id: u16) -> bool {
    (KEY_ITEM_FIRST_ID..=KEY_ITEM_LAST_ID).contains(&item_id)
}
