use crate::side_view::{SIDE_VIEW_DOORS, SideViewDoor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DoorMoveFacts<'a> {
    pub area: &'a str,
    pub moved_in_side_view: bool,
    pub moved_door: u16,
    pub target_in_side_view: bool,
    pub target_door: u16,
}

pub fn door_by_id(area: &str, id: u16) -> Option<&'static SideViewDoor> {
    SIDE_VIEW_DOORS
        .iter()
        .find(|door| door.id == id && door.area == area)
}

pub fn door_entered_by(area: &str, callback: &str) -> Option<&'static SideViewDoor> {
    SIDE_VIEW_DOORS
        .iter()
        .find(|door| door.area == area && door.entry_callback == callback)
}

pub fn door_with_start_point(area: &str, point: i32) -> Option<&'static SideViewDoor> {
    SIDE_VIEW_DOORS
        .iter()
        .find(|door| door.area == area && door.indoor_start_points.contains(&point))
}

/// The door callbacks to run on a player the mod moves across a side view: the target place's
/// entry steps on the way in, the left place's exit steps on the way out, none in any other case.
pub fn door_steps_for_a_move(facts: &DoorMoveFacts) -> &'static [&'static str] {
    match (facts.moved_in_side_view, facts.target_in_side_view) {
        (false, true) => {
            door_by_id(facts.area, facts.target_door).map_or(&[], |door| door.entry_steps)
        }
        (true, false) => {
            door_by_id(facts.area, facts.moved_door).map_or(&[], |door| door.exit_steps)
        }
        _ => &[],
    }
}
