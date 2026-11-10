use serde::{Deserialize, Serialize};

use crate::wire::{
    MAX_CONTAINER_FLAGS, MAX_CONTAINER_GOLD, MAX_CONTAINER_ITEMS, MAX_ITEM_COUNT, MAX_ITEM_ID,
    MAX_STORY_FLAG_ID, StoryArray, WireError,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerItem {
    pub item_id: u16,
    pub count: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerFlag {
    pub array: StoryArray,
    pub id: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerOpened {
    pub phase: u16,
    pub action_id: u16,
    pub items: Vec<ContainerItem>,
    pub gold: u32,
    pub flags: Vec<ContainerFlag>,
}

impl ContainerOpened {
    pub fn validate(&self) -> Result<(), WireError> {
        if self.items.len() > MAX_CONTAINER_ITEMS {
            return Err(WireError::OutOfRange { field: "items" });
        }
        if self.flags.len() > MAX_CONTAINER_FLAGS {
            return Err(WireError::OutOfRange { field: "flags" });
        }
        if self.gold > MAX_CONTAINER_GOLD {
            return Err(WireError::OutOfRange { field: "gold" });
        }
        for item in &self.items {
            if item.item_id > MAX_ITEM_ID {
                return Err(WireError::OutOfRange { field: "item_id" });
            }
            if item.count == 0 || item.count > MAX_ITEM_COUNT {
                return Err(WireError::OutOfRange { field: "count" });
            }
        }
        for flag in &self.flags {
            if !flag.array.holds_flags() || flag.id > MAX_STORY_FLAG_ID {
                return Err(WireError::OutOfRange { field: "flag" });
            }
        }
        Ok(())
    }
}
