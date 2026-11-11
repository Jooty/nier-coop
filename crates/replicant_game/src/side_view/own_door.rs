use replicant_proto::NO_SIDE_VIEW_DOOR;

use crate::side_view::door_entered_by;

/// Which listed door turned this player's side view on, or none: the view is off, or something
/// that is not a listed door (a trap, a scene) turned it on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnSideViewDoor {
    door: u16,
}

impl OwnSideViewDoor {
    pub const fn new() -> Self {
        Self {
            door: NO_SIDE_VIEW_DOOR,
        }
    }

    pub fn door(&self) -> u16 {
        self.door
    }

    pub fn view_flipped(&mut self, now_on: bool, area: Option<&str>, by_callback: &str) {
        self.door = if now_on {
            area.and_then(|area| door_entered_by(area, by_callback))
                .map_or(NO_SIDE_VIEW_DOOR, |door| door.id)
        } else {
            NO_SIDE_VIEW_DOOR
        };
    }
}

impl Default for OwnSideViewDoor {
    fn default() -> Self {
        Self::new()
    }
}
