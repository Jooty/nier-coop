use automata_game::address::{
    INVENTORY_ADD_FUNCTION, INVENTORY_OBJECT, ITEM_NAME_TEXT_GROUP_FUNCTION,
    ITEM_NAME_TEXT_ID_FUNCTION, ITEM_RECORD_COUNT_VALUE, ITEM_RECORDS_POINTER_VALUE,
    UI_TEXT_LOOKUP_FUNCTION, UI_TEXT_TABLE_OBJECT,
};
use automata_game::pickups::{
    ITEM_NAME_MAX_UNITS, ITEM_RECORD_KIND_OFFSET, ITEM_RECORD_SIZE, ITEM_RECORDS_READ_AT_MOST,
    find_item_record,
};

use crate::game_calls::GameCalls;

type ItemTextCall = unsafe extern "system" fn(i32) -> u32;
type TextLookup = unsafe extern "system" fn(usize, u32, u32) -> *const u16;
type InventoryAdd = unsafe extern "system" fn(usize, i32, i32) -> i32;

impl GameCalls {
    /// The kind in the game's item table of one item, or None when the table has no record of it.
    pub fn item_kind(&self, item_id: i32) -> Option<i32> {
        let records = self.read_u64(ITEM_RECORDS_POINTER_VALUE).ok()?;
        let count = self.read_i32(ITEM_RECORD_COUNT_VALUE).ok()?;
        if records == 0 || !(0..=ITEM_RECORDS_READ_AT_MOST).contains(&count) {
            return None;
        }
        let record_at = |index: i32| records + u64::try_from(index).unwrap_or(0) * ITEM_RECORD_SIZE;
        let index = find_item_record(count, item_id, |index| {
            self.read_i32_at(record_at(index)).ok()
        })?;
        self.read_i32_at(record_at(index) + ITEM_RECORD_KIND_OFFSET)
            .ok()
    }

    /// Game thread only: the item's name in the game's own language, as its item lines show it.
    #[expect(
        unsafe_code,
        reason = "calls the game's item name text calls and its UI text lookup"
    )]
    pub fn item_name(&self, item_id: i32) -> Option<String> {
        // SAFETY: the three addresses are the item name text id and group calls and the UI text
        // lookup of the verified build, whose starts were checked at install; the first two take
        // only the item id and the lookup takes the game's own text table, as the game calls them.
        let text = unsafe {
            let text_id = core::mem::transmute::<usize, ItemTextCall>(
                self.address_of(ITEM_NAME_TEXT_ID_FUNCTION),
            )(item_id);
            let group = core::mem::transmute::<usize, ItemTextCall>(
                self.address_of(ITEM_NAME_TEXT_GROUP_FUNCTION),
            )(item_id);
            core::mem::transmute::<usize, TextLookup>(self.address_of(UI_TEXT_LOOKUP_FUNCTION))(
                self.address_of(UI_TEXT_TABLE_OBJECT),
                text_id,
                group,
            )
        };
        if text.is_null() {
            return None;
        }
        let mut units = Vec::new();
        for index in 0..ITEM_NAME_MAX_UNITS {
            let bytes = self.read_bytes_at(text as u64 + 2 * index as u64, 2).ok()?;
            let unit = u16::from_le_bytes([bytes[0], bytes[1]]);
            if unit == 0 {
                break;
            }
            units.push(unit);
        }
        Some(String::from_utf16_lossy(&units))
    }

    /// Game thread only: adds one of the item to this player's inventory; false when the game
    /// refused it (already at its most, no free slot).
    #[expect(unsafe_code, reason = "calls the game's inventory add")]
    pub fn add_one_to_inventory(&self, item_id: i32) -> bool {
        // SAFETY: the address is the inventory add of the verified build, whose start was checked
        // at install, called with the game's own inventory object, one item id and a count of one.
        unsafe {
            core::mem::transmute::<usize, InventoryAdd>(self.address_of(INVENTORY_ADD_FUNCTION))(
                self.address_of(INVENTORY_OBJECT),
                item_id,
                1,
            ) != 0
        }
    }
}
