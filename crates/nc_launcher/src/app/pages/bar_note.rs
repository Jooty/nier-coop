use nc_launcher_core::{StateLineInputs, party_misfit_note, session_state_line};

use crate::app::{Launcher, NetForms};
use crate::net::NetState;

const STARTING_LOBBY_TEXT: &str = "Starting the lobby";
const TESTING_CONNECTION_TEXT: &str = "Testing the connection";
const SENDING_THE_SEAT_TEXT: &str = "Sending the guest's save";

/// The one thing the lobby's bar says, when there is something to say: a failure or a block in warm ink, a
/// wait in muted ink.
pub(super) struct BarNote {
    pub text: String,
    pub warm: bool,
    pub progress: Option<(u16, u16)>,
}

pub(super) fn bar_note(launcher: &Launcher, forms: &NetForms, state: &NetState) -> Option<BarNote> {
    let warm = |text: String| {
        Some(BarNote {
            text,
            warm: true,
            progress: None,
        })
    };
    let calm = |text: &str| {
        Some(BarNote {
            text: text.to_owned(),
            warm: false,
            progress: None,
        })
    };
    if let Some(error) = &launcher.play_error {
        return warm(error.clone());
    }
    let idle = matches!(state, NetState::Idle | NetState::Failed(_));
    if idle {
        if let Some(Err(error)) = &forms.host_result {
            return warm(error.to_string());
        }
        if let Some(Err(error)) = &forms.join_result {
            return warm(error.to_string());
        }
        if forms.hosting_pending() {
            return calm(STARTING_LOBBY_TEXT);
        }
        if forms.joining_pending() {
            return calm(TESTING_CONNECTION_TEXT);
        }
    }
    if let Some(notice) = launcher.follow_notice {
        return warm(notice.text().to_owned());
    }
    if let Some(notice) = launcher.character_bump_notice() {
        return warm(notice);
    }
    let guest = launcher.link.guest;
    if (idle || matches!(state, NetState::Connected { .. }))
        && !launcher.own_lobby_ready().ready
        && let Some(reason) = launcher.lobby_block_reason(guest)
    {
        return warm(reason);
    }
    if let Some(note) = party_misfit_note(1 + launcher.link.members.len(), launcher.link.rules) {
        return warm(note);
    }
    if let Some(progress) = launcher.seat_send_progress() {
        return Some(BarNote {
            text: SENDING_THE_SEAT_TEXT.to_owned(),
            warm: false,
            progress: Some(progress),
        });
    }
    let line = session_state_line(StateLineInputs {
        state,
        own_ticket: launcher.link.ticket.is_some(),
        started: launcher.lobby_started,
        partner_in_play: launcher.link.partner_readiness().is_in_play(),
    })?;
    Some(BarNote {
        text: line.text,
        warm: line.failed,
        progress: None,
    })
}
