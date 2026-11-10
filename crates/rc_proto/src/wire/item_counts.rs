use serde::{Deserialize, Serialize};

use crate::wire::{MAX_ITEM_COUNT, MAX_ITEM_ID, WireError};

pub const MAX_ITEM_COUNT_ENTRIES: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemCountEntry {
    pub item_id: u16,
    pub count: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemCounts {
    pub replaces_all: bool,
    pub entries: Vec<ItemCountEntry>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemTake {
    pub item_id: u16,
    pub count: u8,
}

impl ItemCounts {
    pub fn validate(&self) -> Result<(), WireError> {
        if self.entries.len() > MAX_ITEM_COUNT_ENTRIES {
            return Err(WireError::OutOfRange { field: "entries" });
        }
        for entry in &self.entries {
            if entry.item_id > MAX_ITEM_ID {
                return Err(WireError::OutOfRange { field: "item_id" });
            }
            if entry.count > MAX_ITEM_COUNT {
                return Err(WireError::OutOfRange { field: "count" });
            }
        }
        Ok(())
    }
}

impl ItemTake {
    pub fn validate(&self) -> Result<(), WireError> {
        if self.item_id > MAX_ITEM_ID {
            return Err(WireError::OutOfRange { field: "item_id" });
        }
        if self.count == 0 || self.count > MAX_ITEM_COUNT {
            return Err(WireError::OutOfRange { field: "count" });
        }
        Ok(())
    }
}
