use serde::{Deserialize, Serialize};

use crate::wire::{ENEMY_GROUP_COUNT, MAX_POSITION_ABS, WireError};

/// Names one movable or breakable map object the same way on both games: its kind (the class's
/// vtable RVA), the script group it was found in and the whole-unit cell it was first seen in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorldObjectKey {
    pub kind_vtable_rva: u32,
    pub group: u16,
    pub start_cell: [i32; 3],
}

/// A world object moved on the sender's game, while the sender pushes or carries it (`held`)
/// or as the last word after the sender let go. From the host it is the object's place; from
/// the guest it asks the host to take it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldObjectMoved {
    pub phase: u16,
    pub key: WorldObjectKey,
    pub position: [f32; 3],
    pub held: bool,
}

/// A world object broke on the sender's game. From the host it is the break; from the guest it
/// asks the host to break its copy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldObjectBroken {
    pub phase: u16,
    pub key: WorldObjectKey,
}

impl WorldObjectKey {
    pub fn validate(&self) -> Result<(), WireError> {
        if self.group >= ENEMY_GROUP_COUNT {
            return Err(WireError::OutOfRange { field: "group" });
        }
        if self
            .start_cell
            .iter()
            .any(|axis| axis.unsigned_abs() as f32 > MAX_POSITION_ABS)
        {
            return Err(WireError::OutOfRange {
                field: "start_cell",
            });
        }
        Ok(())
    }
}

impl WorldObjectMoved {
    pub fn validate(&self) -> Result<(), WireError> {
        self.key.validate()?;
        if self
            .position
            .iter()
            .any(|axis| !axis.is_finite() || axis.abs() > MAX_POSITION_ABS)
        {
            return Err(WireError::OutOfRange { field: "position" });
        }
        Ok(())
    }
}

impl WorldObjectBroken {
    pub fn validate(&self) -> Result<(), WireError> {
        self.key.validate()
    }
}
