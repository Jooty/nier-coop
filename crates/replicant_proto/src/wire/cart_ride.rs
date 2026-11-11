use serde::{Deserialize, Serialize};

use crate::wire::{
    MAX_CART_RIDER_OFFSET_ABS, MAX_CART_STATE, MAX_DOOR_LAYOUT_ID, MAX_POSITION_ABS, WireError,
};

/// A cart the host rides changed state: `to_state` 5 to 8 is a ride that just started from
/// parked state `from_state`, 1 to 4 a ride that just ended there. `cart_position` is where the
/// cart was parked before a start, or where it stopped at an end.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CartRide {
    pub phase: u16,
    pub layout_id: Option<u16>,
    pub cart_position: [f32; 3],
    pub from_state: u8,
    pub to_state: u8,
    pub rider_offset: [f32; 3],
}

impl CartRide {
    pub fn validate(&self) -> Result<(), WireError> {
        if self
            .layout_id
            .is_some_and(|layout_id| layout_id > MAX_DOOR_LAYOUT_ID)
        {
            return Err(WireError::OutOfRange { field: "layout_id" });
        }
        if [self.from_state, self.to_state]
            .iter()
            .any(|state| *state == 0 || *state > MAX_CART_STATE)
        {
            return Err(WireError::OutOfRange {
                field: "cart_state",
            });
        }
        if self
            .cart_position
            .iter()
            .any(|axis| !axis.is_finite() || axis.abs() > MAX_POSITION_ABS)
        {
            return Err(WireError::OutOfRange {
                field: "cart_position",
            });
        }
        if self
            .rider_offset
            .iter()
            .any(|axis| !axis.is_finite() || axis.abs() > MAX_CART_RIDER_OFFSET_ABS)
        {
            return Err(WireError::OutOfRange {
                field: "rider_offset",
            });
        }
        Ok(())
    }
}
