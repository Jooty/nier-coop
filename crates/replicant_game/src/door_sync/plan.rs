use replicant_proto::{DoorAction, DoorChange, DoorState};

pub fn plan_door_state(
    current_open: bool,
    current_locked: bool,
    wanted: &DoorState,
) -> Vec<(DoorAction, bool)> {
    let mut steps = Vec::new();
    let moves = current_open != wanted.open;
    let unlock_first = current_locked && (!wanted.locked || moves);
    if unlock_first {
        steps.push((DoorAction::Lock, false));
    }
    if moves {
        let action = if wanted.open {
            DoorAction::Open
        } else {
            DoorAction::Close
        };
        steps.push((action, true));
    }
    if wanted.locked && (!current_locked || unlock_first) {
        steps.push((DoorAction::Lock, true));
    }
    steps
}

pub fn door_reached_state(open: bool, locked: bool, wanted: &DoorState) -> bool {
    open == wanted.open && locked == wanted.locked
}

pub fn changes_for_resulting_state(phase: u16, state: &DoorState) -> Vec<DoorChange> {
    // Planned from the opposite state so that both the open and the lock step are always listed.
    plan_door_state(!state.open, !state.locked, state)
        .into_iter()
        .map(|(action, flag)| DoorChange {
            layout_id: state.layout_id,
            action,
            flag,
            phase,
        })
        .collect()
}

pub fn sender_phase_matches_local(sender_phase: u16, local_phase: Option<u16>) -> bool {
    local_phase == Some(sender_phase)
}
