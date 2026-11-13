use crate::event::status::NetState;

/// What the lobby's bottom bar says about the connection, when there is something to say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateLine {
    pub text: String,
    pub failed: bool,
}

/// The session facts the state line is decided from.
#[derive(Debug, Clone, Copy)]
pub struct StateLineInputs<'a> {
    pub state: &'a NetState,
    pub own_ticket: bool,
    pub started: bool,
    pub partner_in_play: bool,
}

pub fn session_state_line(inputs: StateLineInputs) -> Option<StateLine> {
    let waiting = |text: String| {
        Some(StateLine {
            text,
            failed: false,
        })
    };
    match inputs.state {
        NetState::Idle => None,
        NetState::Joining => waiting("Joining the host".to_owned()),
        NetState::Hosting { .. } => waiting("Waiting for a player to join".to_owned()),
        NetState::Failed(reason) => Some(StateLine {
            text: format!("Failed: {reason}"),
            failed: true,
        }),
        NetState::Connected { peer_name } if inputs.started && !inputs.partner_in_play => {
            if inputs.own_ticket {
                waiting(format!("Waiting for {peer_name}'s game"))
            } else {
                waiting("Waiting for the host's game".to_owned())
            }
        }
        NetState::Connected { .. } => None,
    }
}
