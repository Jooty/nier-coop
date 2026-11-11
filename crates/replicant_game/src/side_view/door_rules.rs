use crate::side_view::{SIDE_VIEW_DOORS, SideViewDoor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DoorMoveFacts<'a> {
    pub area: &'a str,
    pub moved_in_side_view: bool,
    pub moved_door: u16,
    pub target_in_side_view: bool,
    pub target_door: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoorMoveAction {
    Nothing,
    RunSteps(&'static [&'static str]),
    ViewOffRoutine(&'static SideViewDoor),
}

impl DoorMoveAction {
    pub fn is_nothing(self) -> bool {
        self == Self::Nothing
    }
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

/// What the mod does for the side view of a player it moves across one: the target place's
/// entry steps on the way in; on the way out the left place's exit steps, or the game's own
/// view-off routine where no exit step may be run. Nothing in any other case.
pub fn door_action_for_a_move(facts: &DoorMoveFacts) -> DoorMoveAction {
    match (facts.moved_in_side_view, facts.target_in_side_view) {
        (false, true) => door_by_id(facts.area, facts.target_door)
            .filter(|door| !door.entry_steps.is_empty())
            .map_or(DoorMoveAction::Nothing, |door| {
                DoorMoveAction::RunSteps(door.entry_steps)
            }),
        (true, false) => {
            door_by_id(facts.area, facts.moved_door).map_or(DoorMoveAction::Nothing, |door| {
                if door.exit_steps.is_empty() {
                    DoorMoveAction::ViewOffRoutine(door)
                } else {
                    DoorMoveAction::RunSteps(door.exit_steps)
                }
            })
        }
        _ => DoorMoveAction::Nothing,
    }
}
