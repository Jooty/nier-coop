use serde::{Deserialize, Serialize};

use crate::wire::WireError;

pub const MAX_PICKUP_PART_GROUP: u16 = 999;

/// A part of an actor's model that a player picked up in a native pickup errand: the
/// partner hides the same part on its own copy of the actor, so both copies count it once.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartPickup {
    pub phase: u16,
    pub actor_handler_rva: u32,
    pub part_group: u16,
}

impl PartPickup {
    pub fn validate(&self) -> Result<(), WireError> {
        if self.part_group > MAX_PICKUP_PART_GROUP {
            return Err(WireError::OutOfRange {
                field: "part_group",
            });
        }
        Ok(())
    }
}
