use serde::{Deserialize, Serialize};

use crate::wire::{MAX_ITEM_COUNT, MAX_ITEM_ID, WireError};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemPickup {
    pub item_id: u16,
    pub count: u8,
}

impl ItemPickup {
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
