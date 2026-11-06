use crate::zone_rules::EXIT_DOORS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExitDoor {
    pub phase: &'static str,
    pub door_id: Option<i32>,
    pub via: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoorKind {
    LeavesTheArea { via: &'static str },
    StaysInTheArea,
}

impl DoorKind {
    /// What the phase script's `PlayerActionDoor` handler does for this door: `door_id` is the door's
    /// map layout id, the number the game hands the handler.
    pub fn of(phase: &str, door_id: i32) -> Self {
        EXIT_DOORS
            .iter()
            .find(|door| door.phase == phase && door.door_id.is_none_or(|id| id == door_id))
            .map_or(Self::StaysInTheArea, |door| Self::LeavesTheArea {
                via: door.via,
            })
    }

    pub fn leaves_the_area(self) -> bool {
        matches!(self, Self::LeavesTheArea { .. })
    }
}
