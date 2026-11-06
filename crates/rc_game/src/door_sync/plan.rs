use rc_proto::{DoorAction, DoorState};

pub fn plan_door_state(
    current_open: bool,
    current_locked: bool,
    wanted: &DoorState,
) -> Vec<(DoorAction, bool)> {
    let mut steps = Vec::new();
    if current_open && !wanted.open {
        steps.push((DoorAction::Close, true));
    }
    if !current_open && wanted.open {
        steps.push((DoorAction::Open, true));
    }
    if current_locked != wanted.locked {
        steps.push((DoorAction::Lock, wanted.locked));
    }
    steps
}
