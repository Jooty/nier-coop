use serde::{Deserialize, Serialize};

use crate::wire::{MAX_DOOR_LAYOUT_ID, MAX_DOORS_PER_REPORT, WireError};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DoorAction {
    Lock,
    Open,
    Close,
    Active,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoorChange {
    pub layout_id: u16,
    pub action: DoorAction,
    pub flag: bool,
    pub phase: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoorState {
    pub layout_id: u16,
    pub open: bool,
    pub locked: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoorReport {
    pub phase: u16,
    pub doors: Vec<DoorState>,
}

impl DoorChange {
    pub fn validate(&self) -> Result<(), WireError> {
        validate_layout_id(self.layout_id)
    }
}

impl DoorReport {
    pub fn validate(&self) -> Result<(), WireError> {
        if self.doors.len() > MAX_DOORS_PER_REPORT {
            return Err(WireError::OutOfRange { field: "doors" });
        }
        self.doors
            .iter()
            .try_for_each(|door| validate_layout_id(door.layout_id))
    }
}

fn validate_layout_id(layout_id: u16) -> Result<(), WireError> {
    if layout_id > MAX_DOOR_LAYOUT_ID {
        return Err(WireError::OutOfRange { field: "layout_id" });
    }
    Ok(())
}
