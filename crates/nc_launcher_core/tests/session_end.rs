use std::thread;
use std::time::Duration;

use nc_launcher_core::{
    AfterSessionEnd, HOST_UNREACHABLE_TEXT, LauncherCore, LauncherEvent, LauncherNet, LinkStatus,
    NetState, SessionEndCause, SessionEndSide, after_session_end, join_again_offered,
    judge_rejection_text, session_end_log_line,
};
use nc_net::LeaveReason;
use replicant_proto::RejectReason;

use crate::bridge::{connect, rig, wait_until};

const QUIET_WAIT: Duration = Duration::from_millis(200);

fn wait_for_end(core: &LauncherCore) -> Option<(SessionEndCause, LinkStatus)> {
    let mut found = None;
    wait_until(|| {
        found = core
            .drain_events()
            .into_iter()
            .find_map(|event| match event {
                LauncherEvent::SessionEnded { cause, last_seen } => Some((cause, last_seen)),
                _ => None,
            });
        found.is_some()
    });
    found
}

#[test]
fn the_players_own_leave_goes_home() {
    let cause = SessionEndCause::PlayerLeft;
    assert_eq!(cause.side(), SessionEndSide::ThisPlayer);
    assert_eq!(after_session_end(cause, true), AfterSessionEnd::Home);
    assert_eq!(after_session_end(cause, false), AfterSessionEnd::Home);
    assert_eq!(cause.reason_line(), None);
}

#[test]
fn a_session_this_launcher_replaced_moves_the_player_nowhere() {
    for cause in [SessionEndCause::Replaced, SessionEndCause::GaveUp] {
        assert_eq!(cause.side(), SessionEndSide::ThisPlayer);
        assert_eq!(after_session_end(cause, true), AfterSessionEnd::StayPut);
        assert_eq!(after_session_end(cause, false), AfterSessionEnd::StayPut);
        assert_eq!(cause.reason_line(), None);
    }
}

#[test]
fn the_host_leaving_keeps_the_lobby_page_ended() {
    let cause = SessionEndCause::from_host_leaving(LeaveReason::Quit);
    assert_eq!(cause, SessionEndCause::HostLeft);
    assert_eq!(cause.side(), SessionEndSide::OtherPlayer);
    assert_eq!(after_session_end(cause, true), AfterSessionEnd::EndedLobby);
    assert!(cause.reason_line().is_some());
}

#[test]
fn the_host_removing_this_player_keeps_the_lobby_page_ended() {
    let cause = SessionEndCause::from_host_leaving(LeaveReason::RemovedByHost);
    assert_eq!(cause, SessionEndCause::RemovedByHost);
    assert_eq!(after_session_end(cause, true), AfterSessionEnd::EndedLobby);
    assert_eq!(
        cause.reason_line(),
        Some(judge_rejection_text(RejectReason::RemovedByHost))
    );
}

#[test]
fn a_refused_join_keeps_the_lobby_page_ended() {
    let cause = SessionEndCause::Refused(RejectReason::SessionFull);
    assert_eq!(cause.side(), SessionEndSide::OtherPlayer);
    assert_eq!(after_session_end(cause, true), AfterSessionEnd::EndedLobby);
    assert_eq!(
        cause.reason_line(),
        Some(judge_rejection_text(RejectReason::SessionFull))
    );
}

#[test]
fn a_failed_link_keeps_the_lobby_page_ended() {
    for reason in [
        LeaveReason::TimedOut,
        LeaveReason::TransportClosed,
        LeaveReason::ProtocolViolation,
    ] {
        let cause = SessionEndCause::from_host_leaving(reason);
        assert_eq!(cause, SessionEndCause::LinkLost);
        assert_eq!(cause.side(), SessionEndSide::Network);
        assert_eq!(after_session_end(cause, true), AfterSessionEnd::EndedLobby);
    }
    let silent = SessionEndCause::HostUnreachable;
    assert_eq!(silent.reason_line(), Some(HOST_UNREACHABLE_TEXT));
    assert_eq!(after_session_end(silent, true), AfterSessionEnd::EndedLobby);
}

#[test]
fn an_end_from_the_other_side_leaves_a_player_off_the_lobby_page_where_they_are() {
    for cause in [
        SessionEndCause::HostLeft,
        SessionEndCause::RemovedByHost,
        SessionEndCause::Refused(RejectReason::BuildMismatch),
        SessionEndCause::LinkLost,
        SessionEndCause::HostUnreachable,
    ] {
        assert_eq!(after_session_end(cause, false), AfterSessionEnd::StayPut);
    }
}

#[test]
fn every_end_writes_what_ended_it_from_which_side_and_where_the_launcher_went() {
    assert_eq!(
        session_end_log_line(SessionEndCause::HostLeft, AfterSessionEnd::EndedLobby),
        "session ended: the host left, from the other side; the launcher kept the lobby page, ended"
    );
    assert_eq!(
        session_end_log_line(SessionEndCause::PlayerLeft, AfterSessionEnd::Home),
        "session ended: you left the lobby, from this side; the launcher went home"
    );
    assert_eq!(
        session_end_log_line(SessionEndCause::LinkLost, AfterSessionEnd::StayPut),
        "session ended: the link to the host was lost, from the network; the launcher stayed where it was"
    );
}

#[test]
fn only_a_guest_with_a_known_ticket_is_offered_to_join_again() {
    assert!(join_again_offered(true, "nc1ticket"));
    assert!(!join_again_offered(true, "  "));
    assert!(!join_again_offered(false, "nc1ticket"));
}

#[test]
fn the_hosts_close_ends_both_sessions_and_the_guest_keeps_the_seats_it_saw() {
    let mut rig = rig();
    connect(&mut rig);
    assert!(wait_until(|| matches!(
        rig.guest.status().state,
        NetState::Connected { .. }
    )));
    rig.host.leave();
    let (cause, _) = wait_for_end(&rig.host).expect("the host's end");
    assert_eq!(cause, SessionEndCause::PlayerLeft);
    let (cause, last_seen) = wait_for_end(&rig.guest).expect("the guest's end");
    assert_ne!(cause.side(), SessionEndSide::ThisPlayer);
    assert!(last_seen.guest);
    assert!(last_seen.member(0).is_some());
    assert!(matches!(rig.guest.status().state, NetState::Failed(_)));
}

#[test]
fn the_host_removing_the_guest_ends_the_guests_session_as_removed() {
    let mut rig = rig();
    connect(&mut rig);
    assert!(wait_until(|| rig.host.link_status().members.len() == 1));
    let slot = rig.host.link_status().members[0].slot;
    rig.host.remove_member(slot);
    let (cause, _) = wait_for_end(&rig.guest).expect("the guest's end");
    assert_eq!(cause, SessionEndCause::RemovedByHost);
}

#[test]
fn a_new_host_replaces_the_open_session() {
    let rig = rig();
    rig.host.host().expect("first ticket");
    rig.host.host().expect("second ticket");
    let (cause, _) = wait_for_end(&rig.host).expect("the replaced session's end");
    assert_eq!(cause, SessionEndCause::Replaced);
}

#[test]
fn giving_up_on_a_join_ends_it_as_given_up() {
    let rig = rig();
    let ticket = rig.host.host().expect("ticket");
    rig.guest.join(&ticket).expect("join starts");
    rig.guest.give_up();
    let (cause, _) = wait_for_end(&rig.guest).expect("the join's end");
    assert_eq!(cause, SessionEndCause::GaveUp);
}

#[test]
fn leaving_with_no_session_reports_no_end() {
    let rig = rig();
    rig.guest.leave();
    thread::sleep(QUIET_WAIT);
    assert!(
        !rig.guest
            .drain_events()
            .iter()
            .any(|event| matches!(event, LauncherEvent::SessionEnded { .. }))
    );
}
