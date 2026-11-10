use std::time::Duration;

use replicant_hook::net::{GhostWish, NetState, decide_ghost};

const TIMEOUT: Duration = Duration::from_secs(3);

fn decide(
    enabled: bool,
    link: &NetState,
    age_ms: Option<u64>,
    local: Option<u16>,
    peer: u16,
) -> GhostWish {
    decide_ghost(
        enabled,
        link,
        age_ms.map(Duration::from_millis),
        TIMEOUT,
        local,
        peer,
    )
}

#[test]
fn a_connected_peer_in_my_phase_gets_a_ghost() {
    let wish = decide(true, &NetState::Connected, Some(40), Some(4), 4);
    assert_eq!(wish, GhostWish::Wanted);
    assert_eq!(wish.why_not(), None);
}

#[test]
fn a_peer_in_another_phase_gets_no_ghost() {
    let wish = decide(true, &NetState::Connected, Some(40), Some(4), 5);
    assert_eq!(wish, GhostWish::DifferentPhase);
}

#[test]
fn a_silent_peer_loses_the_ghost() {
    let wish = decide(true, &NetState::Connected, Some(3001), Some(4), 4);
    assert_eq!(wish, GhostWish::PeerSilent);
}

#[test]
fn no_connection_or_no_state_means_no_ghost() {
    assert_eq!(
        decide(true, &NetState::LauncherLinked, Some(10), Some(4), 4),
        GhostWish::NotConnected
    );
    assert_eq!(
        decide(true, &NetState::Connected, None, Some(4), 4),
        GhostWish::NotConnected
    );
}

#[test]
fn the_checkbox_beats_everything() {
    assert_eq!(
        decide(false, &NetState::Connected, Some(10), Some(4), 4),
        GhostWish::TurnedOff
    );
}

#[test]
fn an_unreadable_own_phase_waits() {
    assert_eq!(
        decide(true, &NetState::Connected, Some(10), None, 4),
        GhostWish::OwnPhaseUnknown
    );
}
