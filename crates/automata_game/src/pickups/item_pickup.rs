use serde::{Deserialize, Serialize};

use crate::pickups::MAX_ITEM_ID;

/// Items one player was given from the world in one frame, told to every other player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemPickup {
    pub item_id: u32,
    pub count: u16,
    pub key_item: bool,
}

impl ItemPickup {
    pub fn is_valid(&self) -> bool {
        self.item_id <= MAX_ITEM_ID && self.count > 0
    }
}
