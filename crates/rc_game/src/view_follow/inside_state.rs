use crate::view_follow::{
    DoorView, SIDE_VIEW_ENV_OFF, SIDE_VIEW_ENV_ON, SIDE_VIEW_PREFIX, VIEW_CHAINS, door_chain_event,
};

pub fn view_after_callback(name: &str, current: Option<&DoorView>) -> Option<DoorView> {
    if let Some(event) = door_chain_event(name) {
        return Some(DoorView {
            chain: event.chain.to_owned(),
            inside: event.inside,
        });
    }
    if name.starts_with(SIDE_VIEW_PREFIX) {
        let from_chain = VIEW_CHAINS.iter().find_map(|known| {
            if known.entry.contains(&name) {
                Some(DoorView {
                    chain: known.chain.to_owned(),
                    inside: true,
                })
            } else if known.exit.contains(&name) {
                Some(DoorView {
                    chain: known.chain.to_owned(),
                    inside: false,
                })
            } else {
                None
            }
        });
        if from_chain.is_some() {
            return from_chain;
        }
    }
    let inside = match name {
        SIDE_VIEW_ENV_ON => true,
        SIDE_VIEW_ENV_OFF => false,
        _ => return None,
    };
    current.map(|view| DoorView {
        chain: view.chain.clone(),
        inside,
    })
}

pub fn view_after_phase_check(
    current: Option<DoorView>,
    last_phase: Option<u16>,
    phase: Option<u16>,
) -> Option<DoorView> {
    match (current, last_phase, phase) {
        (Some(view), Some(before), Some(now)) if before != now && view.inside => Some(DoorView {
            chain: view.chain,
            inside: false,
        }),
        (view, _, _) => view,
    }
}
