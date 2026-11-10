use serde::{Deserialize, Serialize};

use crate::wire::{MAX_WEAPON_ID, MAX_WEAPON_LEVEL, WireError};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeaponState {
    pub id: u8,
    pub level: u8,
    pub in_hand: bool,
    pub visible: bool,
}

impl WeaponState {
    pub fn validate(&self) -> Result<(), WireError> {
        if self.id > MAX_WEAPON_ID {
            return Err(WireError::OutOfRange { field: "weapon_id" });
        }
        if self.level > MAX_WEAPON_LEVEL {
            return Err(WireError::OutOfRange {
                field: "weapon_level",
            });
        }
        Ok(())
    }
}
