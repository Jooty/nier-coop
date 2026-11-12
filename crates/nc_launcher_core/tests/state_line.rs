use nc_launcher_core::{NetState, StateLineInputs, session_state_line};

fn line_for(state: &NetState, started: bool, partner_in_play: bool) -> Option<(String, bool)> {
    session_state_line(StateLineInputs {
        state,
        own_ticket: true,
        started,
        partner_in_play,
    })
    .map(|line| (line.text, line.failed))
}

#[test]
fn nothing_is_said_when_idle_or_playing_together() {
    assert_eq!(line_for(&NetState::Idle, false, false), None);
    let connected = NetState::Connected {
        peer_name: "Mira".to_owned(),
    };
    assert_eq!(line_for(&connected, false, false), None);
    assert_eq!(line_for(&connected, true, true), None);
}

#[test]
fn a_failure_says_why_and_is_marked_failed() {
    let line = line_for(&NetState::Failed("the host left".to_owned()), false, false);
    assert_eq!(line, Some(("Failed: the host left".to_owned(), true)));
}

#[test]
fn a_started_game_waits_for_the_partner_by_name() {
    let connected = NetState::Connected {
        peer_name: "Mira".to_owned(),
    };
    let line = line_for(&connected, true, false);
    assert_eq!(line, Some(("Waiting for Mira's game".to_owned(), false)));
}

#[test]
fn hosting_alone_waits_for_a_player() {
    let hosting = NetState::Hosting {
        ticket: "nc1".to_owned(),
    };
    assert_eq!(
        line_for(&hosting, false, false).map(|(text, _)| text),
        Some("Waiting for a player to join".to_owned())
    );
}
